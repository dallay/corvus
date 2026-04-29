import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const stableStringCollator = new Intl.Collator("en", {
  numeric: true,
  sensitivity: "base",
});

function sortStrings(values) {
  return [...values].sort((left, right) => stableStringCollator.compare(left, right));
}

function readJson(path) {
  return JSON.parse(fs.readFileSync(path, "utf8"));
}

function readText(path) {
  return fs.readFileSync(path, "utf8");
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function assertIncludesAll(text, patterns, label) {
  for (const pattern of patterns) {
    assert.match(text, pattern, `${label} is missing ${pattern}`);
  }
}

function assertContainsInOrder(text, snippets, label) {
  let cursor = 0;
  for (const snippet of snippets) {
    const nextIndex = text.indexOf(snippet, cursor);
    assert.notEqual(nextIndex, -1, `${label} is missing ${snippet}`);
    cursor = nextIndex + snippet.length;
  }
}

function runReleaseComponentResolver(manualChangedFiles, extraEnv = {}, options = {}) {
  const output = execFileSync("node", ["scripts/resolve-release-components.mjs"], {
    cwd: process.cwd(),
    encoding: "utf8",
    stdio: options.stdio ?? ["ignore", "pipe", "pipe"],
    env: {
      ...process.env,
      EVENT_NAME: "workflow_dispatch",
      MANUAL_CHANGED_FILES: manualChangedFiles.join("\n"),
      ...extraEnv,
    },
  });

  return JSON.parse(output);
}

function runInternalReleaseSync(args = [], options = {}) {
  return execFileSync("node", ["scripts/sync-internal-release-deps.mjs", ...args], {
    cwd: process.cwd(),
    encoding: "utf8",
    stdio: options.stdio ?? ["ignore", "pipe", "pipe"],
    env: {
      ...process.env,
      ...options.env,
    },
  });
}

function runInternalReleaseSyncFailure(args = [], options = {}) {
  try {
    runInternalReleaseSync(args, options);
    throw new Error(`Expected sync script to fail for args: ${args.join(" ")}`);
  } catch (error) {
    if (error?.status === 0) {
      throw error;
    }
    return `${error.stdout ?? ""}${error.stderr ?? ""}`;
  }
}

function withPatchedFile(filePath, transform, callback) {
  const original = readText(filePath);
  const updated = transform(original);
  assert.notEqual(updated, original, `${filePath} patch did not change file contents`);
  fs.writeFileSync(filePath, updated, "utf8");
  try {
    return callback();
  } finally {
    fs.writeFileSync(filePath, original, "utf8");
  }
}

function trustedExecutableDirs() {
  return [
    process.env.HOME && path.join(process.env.HOME, ".cargo", "bin"),
    "/usr/bin",
    "/usr/local/bin",
    "/opt/homebrew/bin",
  ].filter(Boolean);
}

function isTrustedExecutablePath(candidatePath) {
  return trustedExecutableDirs().some((trustedDir) => {
    const relativePath = path.relative(trustedDir, candidatePath);
    return relativePath === path.basename(candidatePath) ||
      (relativePath && !relativePath.startsWith("..") && !path.isAbsolute(relativePath));
  });
}

function resolveExecutable(executableName) {
  const configuredPath = process.env[executableName.toUpperCase()];
  const configuredCandidates =
    typeof configuredPath === "string" && configuredPath.trim()
      ? path.isAbsolute(configuredPath)
        ? isTrustedExecutablePath(configuredPath)
          ? [configuredPath]
          : []
        : !configuredPath.includes(path.sep) && !configuredPath.includes("/")
          ? trustedExecutableDirs().map((trustedDir) => path.join(trustedDir, configuredPath))
          : []
      : [];
  const candidatePaths = [
    ...configuredCandidates,
    ...trustedExecutableDirs().map((trustedDir) => path.join(trustedDir, executableName)),
  ].filter(Boolean);

  return candidatePaths.find((candidatePath) => {
    if (!path.isAbsolute(candidatePath) || !isTrustedExecutablePath(candidatePath)) {
      return false;
    }

    try {
      if (!fs.statSync(candidatePath).isFile()) {
        return false;
      }
      fs.accessSync(candidatePath, fs.constants.X_OK);
      return true;
    } catch {
      return false;
    }
  });
}

const cargoExecutable = resolveExecutable("cargo");
const releaseVersion = readText("clients/cerebro/Cargo.toml").match(/^version\s*=\s*"([^"]+)"$/m)?.[1];

assert.ok(releaseVersion, "Failed to resolve releaseVersion from clients/cerebro/Cargo.toml");

const contractDocs = [
  ".github/workflows/README.md",
  "clients/web/apps/docs/src/content/docs/guides/release.md",
  "clients/web/apps/docs/src/content/docs/es/guides/release.md",
  "clients/web/apps/docs/src/content/docs/clients/agent-runtime/ci-map.md",
  "clients/web/apps/docs/src/content/docs/es/clients/agent-runtime/ci-map.md",
  "CHANGELOG.md",
];

test("release component graph config defines the canonical managed component set", () => {
  const graph = readJson("config/release-components.json");
  const components = Object.keys(graph.components ?? {});

  assert.deepEqual(sortStrings(components), [
    "cerebro",
    "corvus-runtime",
    "gradle-kmp",
    "rook",
  ]);
  assert.equal(graph.components["gradle-kmp"]?.publishPolicy, "validate-only");
  assert.deepEqual(graph.components["corvus-runtime"]?.dependsOnReleaseOf, ["cerebro"]);
  assert.ok(graph.nonReleasePaths.includes("clients/web/"));
});

test("release component graph defines internal release dependency sync edges", () => {
  const graph = readJson("config/release-components.json");
  const edges = graph.internalReleaseDependencies ?? [];

  assert.deepEqual(edges, [
    {
      dependentComponent: "corvus-runtime",
      upstreamComponent: "cerebro",
      manifestPath: "clients/agent-runtime/Cargo.toml",
      dependencyName: "cerebro",
      dependencyPath: "../../clients/cerebro",
      versionSelector: "package.version",
      mode: "must-match-release-version",
      notes: "corvus-runtime ships a versioned path dependency on cerebro",
    },
  ]);
});

test("internal release dependency sync check passes when manifests are aligned", () => {
  const output = runInternalReleaseSync(["--check"]);

  assert.match(output, /All internal release dependencies are aligned/);
  assert.match(output, /corvus-runtime -> cerebro/);
});

test("internal release dependency sync supports write mode flag", () => {
  const help = runInternalReleaseSync(["--help"]);

  assert.match(help, /--check/);
  assert.match(help, /--write/);
});

test("internal release dependency sync write mode is idempotent on aligned manifests", () => {
  const output = runInternalReleaseSync(["--write"]);

  assert.match(output, /No internal release dependency updates were required/);
});

test("release component graph loader exposes internal release dependency metadata", async () => {
  const { loadReleaseComponents } = await import(`../scripts/release-components.mjs?sync=${Date.now()}`);
  const graph = loadReleaseComponents();

  assert.equal(graph.internalReleaseDependencies.length, 1);
  assert.equal(graph.internalReleaseDependencies[0].dependencyName, "cerebro");
  assert.equal(graph.internalReleaseDependencies[0].dependencyPath, "../../clients/cerebro");
});

test("internal release dependency sync check fails on version drift", () => {
  const alignedDependency = `cerebro = { version = "${releaseVersion}", path = "../../clients/cerebro" }`;
  const output = withPatchedFile("clients/agent-runtime/Cargo.toml", (text) =>
    text.replace(alignedDependency, 'cerebro = { version = "0.0.0", path = "../../clients/cerebro" }'),
  () => runInternalReleaseSyncFailure(["--check"]));

  assert.match(output, /version-drift:/);
  assert.ok(output.includes(`expected ${releaseVersion} but found 0.0.0`));
});

test("internal release dependency sync write mode rewrites version drift", () => {
  const alignedDependency = `cerebro = { version = "${releaseVersion}", path = "../../clients/cerebro" }`;
  withPatchedFile("clients/agent-runtime/Cargo.toml", (text) =>
    text.replace(alignedDependency, 'cerebro = { version = "0.0.0", path = "../../clients/cerebro" }'),
  () => {
    const output = runInternalReleaseSync(["--write"]);
    const manifest = readText("clients/agent-runtime/Cargo.toml");

    assert.ok(output.includes(`0.0.0 -> ${releaseVersion}`));
    assert.match(output, /written successfully/);
    assert.ok(manifest.includes(alignedDependency));
  });
});

test("internal release dependency sync fails on path mismatch in both modes", () => {
  const alignedDependency = `cerebro = { version = "${releaseVersion}", path = "../../clients/cerebro" }`;
  withPatchedFile("clients/agent-runtime/Cargo.toml", (text) =>
    text.replace(alignedDependency, `cerebro = { version = "${releaseVersion}", path = "../../clients/not-cerebro" }`),
  () => {
    const checkOutput = runInternalReleaseSyncFailure(["--check"]);
    const writeOutput = runInternalReleaseSyncFailure(["--write"]);

    assert.match(checkOutput, /path-mismatch:/);
    assert.match(writeOutput, /path-mismatch:/);
  });
});

test("internal release dependency sync fails when expected dependency entry is missing", () => {
  const output = withPatchedFile("clients/agent-runtime/Cargo.toml", (text) =>
    text.replace(/^cerebro\s*=\s*\{[^\n]*\}\n/m, ""),
  () => runInternalReleaseSyncFailure(["--check"]));

  assert.match(output, /missing-dependency-entry:/);
  assert.match(output, /cerebro not found/);
});

test("internal release dependency sync flags unmanaged internal path edges", () => {
  const output = withPatchedFile("clients/agent-runtime/Cargo.toml", (text) =>
    text.replace('[dependencies]\n', `[dependencies]\nrogue-cerebro = { path = "../../clients/cerebro", version = "${releaseVersion}", package = "cerebro" }\n`),
  () => runInternalReleaseSyncFailure(["--check"]));

  assert.match(output, /unmanaged-internal-release-edge:/);
  assert.match(output, /rogue-cerebro/);
});

test("internal release dependency sync write mode still reports unmanaged internal path edges", () => {
  const output = withPatchedFile("clients/agent-runtime/Cargo.toml", (text) =>
    text.replace('[dependencies]\n', `[dependencies]\nrogue-cerebro = { path = "../../clients/cerebro", version = "${releaseVersion}", package = "cerebro" }\n`),
  () => runInternalReleaseSyncFailure(["--write"]));

  assert.match(output, /unmanaged-internal-release-edge:/);
  assert.match(output, /rogue-cerebro/);
});

test("internal release dependency sync scans release-managed manifests beyond declared edges", () => {
  const originalGraph = readText("config/release-components.json");
  const originalRookManifest = readText("clients/rook/Cargo.toml");
  const rogueDependency = `cerebro = { path = "../../clients/cerebro", version = "${releaseVersion}" }\n`;

  const updatedGraph = originalGraph.replace(
    '      "ownedPaths": [\n        "clients/rook/"\n      ],',
    '      "ownedPaths": [\n        "clients/rook/"\n      ],\n      "versionSurfaces": [\n        "version.txt",\n        "clients/rook/Cargo.toml",\n        "clients/rook/npm/rook/package.json"\n      ],',
  );
  const updatedRookManifest = originalRookManifest.replace('[dependencies]\n', `[dependencies]\n${rogueDependency}`);

  fs.writeFileSync("config/release-components.json", updatedGraph, "utf8");
  fs.writeFileSync("clients/rook/Cargo.toml", updatedRookManifest, "utf8");

  try {
    const output = runInternalReleaseSyncFailure(["--check"]);
    assert.match(output, /unmanaged-internal-release-edge:/);
    assert.match(output, /clients\/rook\/Cargo\.toml/);
    assert.match(output, /cerebro/);
  } finally {
    fs.writeFileSync("config/release-components.json", originalGraph, "utf8");
    fs.writeFileSync("clients/rook/Cargo.toml", originalRookManifest, "utf8");
  }
});

test("pull-request checks validate internal release dependency sync before Cargo lockfiles", () => {
  const workflow = readText(".github/workflows/pull-request-check.yml");

  assertContainsInOrder(
    workflow,
    [
      "- name: 🔍 Check internal release dependency sync",
      "node scripts/sync-internal-release-deps.mjs --check",
      "- name: 🦀 Check Rust lockfiles are up to date",
    ],
    "pull-request-check.yml",
  );
});

test("stable release workflow normalizes and persists internal dependency sync after release-please", () => {
  const workflow = readText(".github/workflows/release-please.yml");

  assertContainsInOrder(
    workflow,
    [
      "- name: 🤖 Run release-please",
      "- name: 🔁 Sync internal release dependencies",
      "node scripts/sync-internal-release-deps.mjs --write",
      "- name: 💾 Commit synced internal release dependencies",
      "SKIP_GIT_HOOKS: \"1\"",
      "git add clients/agent-runtime/Cargo.toml clients/cerebro/Cargo.toml",
      "git commit -m \"chore: sync internal release dependencies\"",
      "git push",
    ],
    "release-please.yml",
  );
});

test("beta release workflow normalizes and persists internal dependency sync after release-please", () => {
  const workflow = readText(".github/workflows/release-please-beta.yml");

  assertContainsInOrder(
    workflow,
    [
      "- name: 🤖 Run release-please",
      "- name: 🔁 Sync internal release dependencies",
      "node scripts/sync-internal-release-deps.mjs --write",
      "- name: 💾 Commit synced internal release dependencies",
      "SKIP_GIT_HOOKS: \"1\"",
      "git add clients/agent-runtime/Cargo.toml clients/cerebro/Cargo.toml",
      "git commit -m \"chore: sync internal release dependencies\"",
      "git push",
    ],
    "release-please-beta.yml",
  );
});

test("sync-cargo-lockfiles runs internal dependency sync before lockfile regeneration", () => {
  const workflow = readText(".github/workflows/sync-cargo-lockfiles.yml");

  assertContainsInOrder(
    workflow,
    [
      "- name: 🔁 Sync internal release dependencies",
      "node scripts/sync-internal-release-deps.mjs --write",
      "- name: 🔄 Regenerate Cargo.lock files",
    ],
    "sync-cargo-lockfiles.yml",
  );
});

test("sync-cargo-lockfiles commit step stages all rewritten manifests and lockfiles", () => {
  const workflow = readText(".github/workflows/sync-cargo-lockfiles.yml");

  assertContainsInOrder(
    workflow,
    [
      "- name: 💾 Commit updated lockfiles",
      "git add --all -- clients/**/*.Cargo.toml clients/**/*.Cargo.lock",
    ],
    "sync-cargo-lockfiles.yml",
  );
});

test("archived openspec state reflects completed apply and verify phases", () => {
  const state = readText("openspec/changes/archive/2026-04-29-release-internal-dependency-sync/state.yaml");

  assert.match(state, /^status: completed$/m);
  assert.match(state, /^  apply:\n    status: completed$/m);
  assert.match(state, /^  verify:\n    status: completed$/m);
});

test("archived verify report no longer instructs archive retry", () => {
  const verifyReport = readText("openspec/changes/archive/2026-04-29-release-internal-dependency-sync/verify-report.md");

  assert.doesNotMatch(verifyReport, /Archive should be retried/);
});

test("openspec design and tasks use canonical internalReleaseDependencies naming", () => {
  const design = readText("openspec/changes/archive/2026-04-29-release-internal-dependency-sync/design.md");
  const tasks = readText("openspec/changes/archive/2026-04-29-release-internal-dependency-sync/tasks.md");

  assert.match(design, /internalReleaseDependencies/);
  assert.match(design, /versionSelector/);
  assert.doesNotMatch(design, /internal_release_dependency/);
  assert.doesNotMatch(design, /version_selector/);
  assert.match(tasks, /internalReleaseDependencies/);
  assert.doesNotMatch(tasks, /internal_release_dependencies/);
});

test("release runbooks describe internal dependency sync diagnostics", () => {
  const english = readText("clients/web/apps/docs/src/content/docs/guides/release.md");
  const spanish = readText("clients/web/apps/docs/src/content/docs/es/guides/release.md");

  assert.match(english, /sync-internal-release-deps\.mjs/);
  assert.match(english, /internal release dependency/i);
  assert.match(spanish, /sync-internal-release-deps\.mjs/);
  assert.match(spanish, /dependenc/i);
});

test("release component graph stays aligned with release-please managed packages", async () => {
  const graph = readJson("config/release-components.json");
  const stableConfig = readJson("release-please-config.json");
  const betaConfig = readJson("release-please-beta-config.json");
  const stableManifest = readJson(".release-please-manifest.json");
  const betaManifest = readJson(".release-please-beta-manifest.json");
  const { loadReleaseComponents } = await import(`../scripts/release-components.mjs?ts=${Date.now()}`);

  const loadedGraph = loadReleaseComponents();
  const graphPublishableComponents = sortStrings(
    Object.entries(loadedGraph.components)
      .filter(([, component]) => component.publishPolicy === "publishable")
      .map(([componentId]) => componentId),
  );
  const stableReleasePleaseComponents = sortStrings(
    Object.values(stableConfig.packages).map((pkg) => pkg.component),
  );
  const betaReleasePleaseComponents = sortStrings(
    Object.values(betaConfig.packages).map((pkg) => pkg.component),
  );
  const stableManifestComponents = sortStrings(Object.keys(stableManifest));
  const betaManifestComponents = sortStrings(Object.keys(betaManifest));

  assert.deepEqual(graphPublishableComponents, ["cerebro", "corvus-runtime", "rook"]);
  assert.deepEqual(stableReleasePleaseComponents, graphPublishableComponents);
  assert.deepEqual(betaReleasePleaseComponents, graphPublishableComponents);
  assert.deepEqual(stableManifestComponents, graphPublishableComponents);
  assert.deepEqual(betaManifestComponents, graphPublishableComponents);
  assert.equal(graph.components["gradle-kmp"]?.publishPolicy, "validate-only");
  assert.ok(!stableManifestComponents.includes("gradle-kmp"));
  assert.ok(!betaManifestComponents.includes("gradle-kmp"));
});

test("release component resolver marks rook-owned paths as direct rook scope", () => {
  const resolved = runReleaseComponentResolver(["clients/rook/src/main.rs"]);

  assert.deepEqual(resolved.affected_components, ["rook"]);
  assert.deepEqual(resolved.direct_components, ["rook"]);
  assert.deepEqual(resolved.transitive_components, []);
  assert.deepEqual(resolved.non_release_paths, []);
  assert.deepEqual(resolved.unmapped_paths, []);
  assert.ok(resolved.reasons.rook.includes("owned:clients/rook/src/main.rs"));
});

test("release component resolver expands cerebro changes to runtime transitively", () => {
  const resolved = runReleaseComponentResolver(["clients/cerebro/src/lib.rs"]);

  assert.deepEqual(resolved.affected_components, ["cerebro", "corvus-runtime"]);
  assert.deepEqual(resolved.direct_components, ["cerebro"]);
  assert.deepEqual(resolved.transitive_components, ["corvus-runtime"]);
  assert.ok(resolved.reasons.cerebro.includes("owned:clients/cerebro/src/lib.rs"));
  assert.ok(resolved.reasons["corvus-runtime"].includes("depends-on-release-of:cerebro"));
});

test("release component resolver fans out shared release infra to declared components", () => {
  const resolved = runReleaseComponentResolver([".github/workflows/_publish.yml"]);

  assert.deepEqual(resolved.affected_components, ["cerebro", "corvus-runtime", "gradle-kmp", "rook"]);
  assert.deepEqual(resolved.direct_components, ["cerebro", "corvus-runtime", "gradle-kmp", "rook"]);
  assert.deepEqual(resolved.transitive_components, []);
  assert.deepEqual(resolved.unmapped_paths, []);
  for (const componentId of resolved.affected_components) {
    assert.ok(
      resolved.reasons[componentId].includes("shared-infra:.github/workflows/_publish.yml"),
      `${componentId} should include shared infra reason`,
    );
  }
});

test("release component resolver classifies web-only changes as non-release", () => {
  const resolved = runReleaseComponentResolver(["clients/web/apps/docs/src/content/docs/guides/release.md"]);

  assert.deepEqual(resolved.affected_components, []);
  assert.deepEqual(resolved.direct_components, []);
  assert.deepEqual(resolved.transitive_components, []);
  assert.deepEqual(resolved.unmapped_paths, []);
  assert.deepEqual(resolved.non_release_paths, ["clients/web/apps/docs/src/content/docs/guides/release.md"]);
});

function runReleaseTagResolver(releaseTag, releaseBody = "", options = {}) {
  const output = execFileSync("node", ["scripts/resolve-release-from-tag.mjs"], {
    cwd: process.cwd(),
    encoding: "utf8",
    stdio: options.stdio ?? ["ignore", "pipe", "pipe"],
    env: {
      ...process.env,
      RELEASE_TAG: releaseTag,
      RELEASE_BODY: releaseBody,
    },
  });

  return JSON.parse(output);
}

test("release tag resolver maps supported component tags", () => {
  const resolved = runReleaseTagResolver("rook-v1.2.3");

  assert.equal(resolved.supported_release, true);
  assert.deepEqual(resolved.affected_components, ["rook"]);
  assert.equal(resolved.resolution_reason, "release tag prefix");
});

test("release tag resolver accepts release body multi-component override", () => {
  const resolved = runReleaseTagResolver(
    "rook-v1.2.3",
    "## Summary\naffected_components: rook, corvus-runtime\n",
  );

  assert.equal(resolved.supported_release, true);
  assert.deepEqual(resolved.affected_components, ["corvus-runtime", "rook"]);
  assert.equal(resolved.resolution_reason, "release body override");
});

function runAffectedComponentsValidator(affectedComponents, options = {}) {
  const output = execFileSync("node", ["scripts/validate-affected-components.mjs"], {
    cwd: process.cwd(),
    encoding: "utf8",
    stdio: options.stdio ?? ["ignore", "pipe", "pipe"],
    env: {
      ...process.env,
      AFFECTED_COMPONENTS: affectedComponents,
    },
  });

  return JSON.parse(output);
}

function getExecFileSyncFailure(fn) {
  try {
    fn();
  } catch (error) {
    const combinedMessage = [error.message, error.stderr, error.stdout]
      .filter(Boolean)
      .map((value) => String(value))
      .join("\n");
    error.combinedMessage = combinedMessage;
    return error;
  }

  throw new Error("Expected command to fail");
}

test("affected components validator accepts publishable component payloads", () => {
  const validated = runAffectedComponentsValidator('["rook","corvus-runtime"]');

  assert.deepEqual(validated.affected_components, ["corvus-runtime", "rook"]);
  assert.equal(validated.has_rook, true);
  assert.equal(validated.has_corvus_runtime, true);
  assert.equal(validated.has_cerebro, false);
  assert.equal(validated.has_gradle_kmp, false);
});

test("release tag resolver marks unsupported tags as non-release", () => {
  const resolved = runReleaseTagResolver("web-v1.2.3");

  assert.equal(resolved.supported_release, false);
  assert.deepEqual(resolved.affected_components, []);
  assert.equal(resolved.resolution_reason, "unsupported release tag");
});

test("release tag resolver rejects invalid release body overrides", () => {
  const error = getExecFileSyncFailure(() =>
    runReleaseTagResolver("rook-v1.2.3", "affected_components: rook, web\n"),
  );

  assert.match(error.combinedMessage, /Unsupported affected_components override in release body: web/);
});

test("affected components validator rejects invalid JSON", () => {
  const error = getExecFileSyncFailure(() => runAffectedComponentsValidator("not-json"));

  assert.match(error.combinedMessage, /Invalid AFFECTED_COMPONENTS payload: not-json/);
});

test("affected components validator rejects empty arrays", () => {
  const error = getExecFileSyncFailure(() => runAffectedComponentsValidator("[]"));

  assert.match(error.combinedMessage, /No publishable affected components were provided to _publish/);
});

test("affected components validator rejects unknown components", () => {
  const error = getExecFileSyncFailure(() => runAffectedComponentsValidator('["rook","web"]'));

  assert.match(error.combinedMessage, /Unknown affected components in _publish input: web/);
});

test("release component resolver hard-fails unmapped paths in strict mode", () => {
  const error = getExecFileSyncFailure(() =>
    runReleaseComponentResolver(["totally-new-surface/file.txt"], { STRICT_RELEASE_GRAPH: "true" }),
  );

  assert.match(error.combinedMessage, /Unmapped release-relevant paths: totally-new-surface\/file\.txt/);
});

test("release-please fan-out only includes shipped stable artifacts", () => {
  const config = readJson("release-please-config.json");
  const runtimePackage = config.packages["clients/agent-runtime"];
  const extraFiles = runtimePackage["extra-files"];
  const filePaths = new Set(extraFiles.map((entry) => entry.path));
  const cargoTomlTargets = new Set(
    extraFiles
      .filter((entry) => entry.path === "clients/agent-runtime/Cargo.toml")
      .map((entry) => entry.jsonpath),
  );
  const optionalDependencyPins = new Set(
    extraFiles
      .filter((entry) => entry.path === "clients/agent-runtime/npm/corvus/package.json")
      .map((entry) => entry.jsonpath),
  );

  assert.equal(config["bootstrap-sha"], undefined);
  assert.equal(config["skip-github-release"], undefined);
  assert.equal(config["skip-changelog"], undefined);
  assert.equal(runtimePackage.component, "corvus-runtime");
  assert.equal(runtimePackage["release-type"], "rust");
  assert.ok(!filePaths.has("clients/web/**/package.json"));
  assert.ok(!filePaths.has("clients/agent-runtime/npm/**/package.json"));
  assert.ok(!filePaths.has("clients/agent-runtime/npm/corvus-cli/package.json"));
  assert.ok(!filePaths.has("clients/agent-runtime/npm/corvus-windows-arm64/package.json"));

  for (const expectedPath of [
    "clients/agent-runtime/Cargo.toml",
    "clients/agent-runtime/npm/corvus/package.json",
    "clients/agent-runtime/npm/corvus-darwin-x64/package.json",
    "clients/agent-runtime/npm/corvus-darwin-arm64/package.json",
    "clients/agent-runtime/npm/corvus-linux-x64/package.json",
    "clients/agent-runtime/npm/corvus-linux-arm64/package.json",
    "clients/agent-runtime/npm/corvus-windows-x64/package.json",
  ]) {
    assert.ok(filePaths.has(expectedPath), `missing version target: ${expectedPath}`);
  }

  assert.ok(
    !optionalDependencyPins.has(
      "$.optionalDependencies['@dallay/corvus-windows-arm64']",
    ),
  );
  assert.ok(cargoTomlTargets.has("$.package.version"));
  assert.ok(config.packages["clients/cerebro"]["extra-files"].some(
    (entry) => entry.path === "clients/cerebro/Cargo.toml" && entry.jsonpath === "$.package.version",
  ));
});

test("beta release-please config reuses shipped artifact fan-out with prerelease semantics", () => {
  const stableConfig = readJson("release-please-config.json");
  const betaConfig = readJson("release-please-beta-config.json");
  const stablePackage = stableConfig.packages["clients/agent-runtime"];
  const betaPackage = betaConfig.packages["clients/agent-runtime"];

  assert.deepEqual(betaPackage["extra-files"], stablePackage["extra-files"]);
  assert.equal(betaPackage["release-type"], stablePackage["release-type"]);
  assert.equal(betaPackage.component, stablePackage.component);
  assert.equal(betaPackage.prerelease, true);
  assert.equal(betaPackage["prerelease-type"], "beta");
  assert.equal(betaPackage.versioning, "prerelease");
  assert.match(betaConfig["pull-request-title-pattern"], /beta/i);
});

test("runtime npm metadata only advertises supported shipped platforms", () => {
  const pkg = readJson("clients/agent-runtime/npm/corvus/package.json");

  assert.deepEqual(sortStrings(Object.keys(pkg.optionalDependencies)), [
    "@dallay/corvus-darwin-arm64",
    "@dallay/corvus-darwin-x64",
    "@dallay/corvus-linux-arm64",
    "@dallay/corvus-linux-x64",
    "@dallay/corvus-windows-x64",
  ]);
});

test("release scope utils centralize stable sorting and publishable component helpers", () => {
  const utilsScript = readText("scripts/release-scope-utils.mjs");

  assert.match(utilsScript, /const stableStringCollator = new Intl\.Collator\("en", \{/);
  assert.match(utilsScript, /numeric: true/);
  assert.match(utilsScript, /sensitivity: "base"/);
  assert.match(utilsScript, /export function sortStrings\(values\) \{/);
  assert.match(utilsScript, /return \[\.\.\.values\]\.sort\(\(left, right\) => stableStringCollator\.compare\(left, right\)\);/);
  assert.match(utilsScript, /export function getPublishableComponentIds\(graph\) \{/);
  assert.match(utilsScript, /component\.publishPolicy === "publishable"/);
  assert.match(utilsScript, /export function getKnownComponentIds\(graph\) \{/);

  for (const scriptPath of [
    "scripts/resolve-release-components.mjs",
    "scripts/resolve-release-from-tag.mjs",
    "scripts/validate-affected-components.mjs",
  ]) {
    const script = readText(scriptPath);
    assert.match(script, /from "\.\/release-scope-utils\.mjs"/);
    assert.doesNotMatch(script, /const stableStringCollator = new Intl\.Collator/);
  }
});


test("release workflows encode release-please-owned stable and beta governance", () => {
  const releasePlease = readText(".github/workflows/release-please.yml");
  const releasePleaseBeta = readText(".github/workflows/release-please-beta.yml");
  const publishRelease = readText(".github/workflows/publish-release.yml");
  const publishSnapshot = readText(".github/workflows/publish-snapshot.yml");
  const publishWorkflow = readText(".github/workflows/_publish.yml");

  assertIncludesAll(
    releasePleaseBeta,
    [
      /id: release-please/,
      /manifest-file: \.release-please-beta-manifest\.json/,
      /prerelease release PR\/tag\/GitHub Release path/i,
      /canonical beta GitHub Release notes/i,
      /node scripts\/resolve-release-components\.mjs/,
      /GITHUB_OUTPUT/,
      /GITHUB_STEP_SUMMARY/,
    ],
    "release-please workflow",
  );
  assert.doesNotMatch(releasePlease, /exclusive = \{/);
  assert.doesNotMatch(releasePlease, /shared_exact = \{/);

  assertContainsInOrder(
    releasePlease,
    [
      "release-please:",
      "permissions:",
      "contents: write",
      "pull-requests: write",
      "issues: write",
    ],
    "release-please workflow",
  );
  assertContainsInOrder(
    releasePlease,
    ["publish-release:", "permissions:", "contents: write", "packages: write"],
    "release-please workflow",
  );
  const jobsSectionIndex = releasePlease.indexOf("jobs:\n");
  assert.notEqual(jobsSectionIndex, -1, "release-please workflow is missing jobs section");
  assert.equal(
    releasePlease.slice(0, jobsSectionIndex).includes("permissions:"),
    false,
    "release-please workflow should not declare top-level permissions",
  );
  assert.doesNotMatch(releasePlease, /secrets:\s+inherit/);

  assertIncludesAll(
    releasePleaseBeta,
    [
      /id: release-please/,
      /config-file: release-please-beta-config\.json/,
      /manifest-file: \.release-please-beta-manifest\.json/,
      /target-branch: beta/,
      /prerelease release PR\/tag\/GitHub Release path/i,
      /canonical beta GitHub Release notes/i,
      /node scripts\/resolve-release-components\.mjs/,
      /GITHUB_OUTPUT/,
      /GITHUB_STEP_SUMMARY/,
    ],
    "release-please-beta workflow",
  );
  assert.doesNotMatch(releasePleaseBeta, /exclusive = \{/);
  assert.doesNotMatch(releasePleaseBeta, /shared_exact = \{/);
  assertContainsInOrder(
    releasePleaseBeta,
    [
      "release-please-beta:",
      "permissions:",
      "contents: write",
      "pull-requests: write",
      "issues: write",
    ],
    "release-please-beta workflow",
  );
  assertContainsInOrder(
    releasePleaseBeta,
    ["publish-beta:", "permissions:", "contents: write", "packages: write"],
    "release-please-beta workflow",
  );
  assert.doesNotMatch(releasePleaseBeta, /secrets:\s+inherit/);

  assertIncludesAll(
    publishRelease,
    [
      /on:\s*release:\s*types:\s*- published/s,
      /Canonical stable release handoff/i,
      /release: true/,
      /prerelease: false/,
      /release_tag: \$\{\{ github\.event\.release\.tag_name \}\}/,
      /release_id: \$\{\{ github\.event\.release\.id \}\}/,
      /attach artifacts to the existing canonical GitHub Release/i,
      /permissions:\s+contents: write\s+packages: write/s,
      /secrets:\s+SIGNING_IN_MEMORY_KEY:/,
      /DOCKERHUB_TOKEN:/,
      /corvus-runtime-v\*/,
      /cerebro-v\*/,
      /rook-v\*/,
      /affected_components:\s*rook, corvus-runtime/,
      /Resolution source:/,
      /node scripts\/resolve-release-from-tag\.mjs/,
      /GITHUB_OUTPUT/,
      /GITHUB_STEP_SUMMARY/,
    ],
    "publish-release workflow",
  );
  assert.doesNotMatch(publishRelease, /override_match = re\.search/);
  assert.doesNotMatch(publishRelease, /supported_components = \{/);
  assert.doesNotMatch(publishRelease, /push:\s*tags:/s);
  assert.doesNotMatch(publishRelease, /changelog:\s*true/);
  assert.doesNotMatch(publishRelease, /secrets:\s+inherit/);

  assertIncludesAll(
    publishSnapshot,
    [
      /Snapshots stay outside stable GitHub Release ownership/,
      /release: false/,
      /prerelease: false/,
      /permissions:\s+contents: read\s+packages: write/s,
      /secrets:\s+SIGNING_IN_MEMORY_KEY:/,
      /DOCKERHUB_TOKEN:/,
    ],
    "publish-snapshot workflow",
  );
  assert.doesNotMatch(publishSnapshot, /changelog:/);
  assert.doesNotMatch(publishSnapshot, /secrets:\s+inherit/);

  assertIncludesAll(
    publishWorkflow,
    [
      /prerelease:/,
      /release_tag:/,
      /release_id:/,
      /release_version/,
      /release_channel/,
      /npm_dist_tag/,
      /has_corvus_runtime/,
      /has_rook/,
      /has_cerebro/,
      /has_gradle_kmp/,
      /AFFECTED_COMPONENTS: \$\{\{ inputs\.affected_components \|\| '\[\]' \}\}/,
      /No version checks configured for affected components:/,
      /Release publish summary/,
      /npm platform publish summary/,
      /npm base publish summary/,
      /Resolved affected components:/,
      /scripts\/validate-affected-components\.mjs/,
      /VALIDATION_OUTPUT: \$\{\{ runner\.temp \}\}\/affected-components\.json/,
    ],
    "publish workflow",
  );
  assert.doesNotMatch(publishWorkflow, /known_components = \{/);
  assert.doesNotMatch(publishWorkflow, /const stableStringCollator/);

  assert.doesNotMatch(publishWorkflow, /release-changelog-builder-action/);
  assert.doesNotMatch(publishWorkflow, /softprops\/action-gh-release/);
  assert.doesNotMatch(publishWorkflow, /\.github\/config\/changelog\.json/);
  assert.doesNotMatch(publishWorkflow, /inputs\.changelog/);
});

test("cargo publish contract keeps local cerebro path and release version aligned", () => {
  const cargoToml = readText("clients/agent-runtime/Cargo.toml");
  const cerebroToml = readText("clients/cerebro/Cargo.toml");

  assert.match(
    cargoToml,
    new RegExp(
      `cerebro = \\{ version = "${escapeRegex(releaseVersion)}", path = "\\.\\.\\/\\.\\.\\/clients\\/cerebro" \\}`,
    ),
  );
  assert.match(cerebroToml, new RegExp(`^version = "${escapeRegex(releaseVersion)}"$`, "m"));
});

test("rust lockfiles stay valid for --locked release commands", (t) => {
  if (!cargoExecutable) {
    if (process.env.CI) {
      assert.fail("cargo executable not found in trusted absolute paths during CI");
    }
    t.skip("cargo executable not found in trusted absolute paths");
    return;
  }

  for (const cwd of ["clients/agent-runtime", "clients/cerebro"]) {
    try {
      execFileSync(cargoExecutable, ["metadata", "--locked", "--format-version", "1"], {
        cwd,
        stdio: "pipe",
        maxBuffer: 1024 * 1024 * 32,
      });
    } catch (error) {
      const stderr = Buffer.isBuffer(error.stderr)
        ? error.stderr.toString("utf8")
        : typeof error.stderr === "string"
          ? error.stderr
          : "";
      if (/Could not resolve host|failed to download from `https:\/\/static\.crates\.io\//i.test(stderr)) {
        t.skip(`cargo metadata requires network access for ${cwd} in this environment`);
        return;
      }
      throw error;
    }
  }
});

test("release docs, changelog, and CI maps describe one stable contract", () => {
  const docsByPath = Object.fromEntries(contractDocs.map((path) => [path, readText(path)]));

  for (const [path, doc] of Object.entries(docsByPath)) {
    assertIncludesAll(
      doc,
      [/release-please/i, /GitHub Release/i, /release\.published/i],
      path,
    );
    assert.doesNotMatch(
      doc,
      /_publish\.yml.*(creates|owns).*(GitHub Release|release notes)/i,
      `${path} still grants _publish ownership of canonical release notes`,
    );
  }

  const docsEn = docsByPath["clients/web/apps/docs/src/content/docs/guides/release.md"];
  const docsEs = docsByPath["clients/web/apps/docs/src/content/docs/es/guides/release.md"];
  const workflowsReadme = docsByPath[".github/workflows/README.md"];
  const ciMap = docsByPath["clients/web/apps/docs/src/content/docs/clients/agent-runtime/ci-map.md"];
  const ciMapEs = docsByPath["clients/web/apps/docs/src/content/docs/es/clients/agent-runtime/ci-map.md"];
  const changelog = docsByPath["CHANGELOG.md"];

  assertIncludesAll(
    ciMap,
    [
      /\.github\/workflows\/publish-release\.yml` \(`Publish Release`\)/,
      /Purpose: publish stable artifacts after the canonical GitHub Release is published/i,
      /\.github\/workflows\/release-please-beta\.yml` \(`Release Please Beta`\)/,
      /Purpose: create beta prerelease PRs, tags, GitHub Releases, and beta artifact publication from the `beta` branch/i,
      /shared release-scope resolvers from `scripts\/resolve-release-components\.mjs` and `scripts\/resolve-release-from-tag\.mjs`/i,
    ],
    "English CI map",
  );

  assertIncludesAll(
    ciMapEs,
    [
      /\.github\/workflows\/publish-release\.yml` \(`Publish Release`\)/,
      /Propósito: publicar artefactos estables después de que se publique el GitHub Release canónico/i,
      /\.github\/workflows\/release-please-beta\.yml` \(`Release Please Beta`\)/,
      /Propósito: crear PRs beta, tags, GitHub Releases y publicación beta desde la rama `beta`/i,
      /resolvers compartidos de release scope desde `scripts\/resolve-release-components\.mjs` y `scripts\/resolve-release-from-tag\.mjs`/i,
    ],
    "Spanish CI map",
  );

  assertIncludesAll(
    docsEs,
    [
      /release-please-beta\.yml/i,
      /rama `beta`/i,
      /dist-tag `beta` de npm/i,
      /paquetes web privados están excluidos/i,
      /recuperación manual/i,
      /por pull request/i,
      /adjuntar assets al GitHub Release existente/i,
    ],
    "Spanish release runbook",
  );
  assertIncludesAll(
    docsEs,
    [
      /`release-please\.yml` es dueño del PR repo-wide de release, del tag canónico `vX\.Y\.Z`, del GitHub Release canónico y de las notas canónicas del release estable\./i,
      /`release-please-beta\.yml` es dueño del PR repo-wide beta, del tag canónico `vX\.Y\.Z-beta\.N`, del GitHub Release beta canónico y de las notas canónicas del release beta desde la rama `beta`\./i,
      /`publish-release\.yml` y `_publish\.yml` solo son dueños de la publicación de artefactos después de que `release-please` publique el GitHub Release\./i,
      /`publish-snapshot\.yml` es una ruta solo de snapshots para Gradle\/Maven y no es dueña de notas de release estables\./i,
      /La automatización estable valida y publica solo artefactos enviados:/i,
      /crate de Rust: `cerebro`/i,
      /crate de Rust \+ paquete npm \+ imagen Docker: `corvus-runtime`/i,
      /crate de Rust \+ paquete npm \+ imagen Docker: `rook`/i,
      /`release-please\.yml` es dueño del PR repo-wide de release, del tag canónico `vX\.Y\.Z`, del GitHub Release canónico y de las notas canónicas del release estable\./i,
      /`publish-release\.yml` y `_publish\.yml` solo son dueños de la publicación de artefactos después de que `release-please` publique el GitHub Release\./i,
      /`release-please-beta\.yml` corre desde la rama `beta` y es dueño de los PRs, tags, GitHub Releases y notas del canal beta\./i,
      /`publish-snapshot\.yml` es una ruta solo de snapshots para Gradle\/Maven y no es dueña de notas de release estables\./i,
      /Solo `release-please` y `release-please-beta` son dueños de las notas canónicas del GitHub Release/i,
      /`publish-release\.yml` y `_publish\.yml` nunca deben reemplazar, editar o reinterpretar las notas canónicas del GitHub Release/i,
      /El workflow beta sigue respetando la misma superficie de artefactos: `cerebro`, `corvus-runtime` y `rook`/i,
      /`config\/release-components\.json` es el graph canónico de componentes gestionados/i,
      /`scripts\/resolve-release-components\.mjs` resuelve el scope por archivos cambiados para `release-please\.yml` y `release-please-beta\.yml`/i,
      /`scripts\/resolve-release-from-tag\.mjs` resuelve el scope del publish estable desde el tag del release y el override opcional `affected_components:`/i,
    ],
    "Spanish release guide",
  );


  assertIncludesAll(
    changelog,
    [
      /GitHub Releases/,
      /release-please/,
      /release\.published/,
    ],
    "CHANGELOG.md",
  );
  for (const path of [
    ".github/workflows/README.md",
    "clients/web/apps/docs/src/content/docs/guides/release.md",
    "clients/web/apps/docs/src/content/docs/es/guides/release.md",
    "CHANGELOG.md",
  ]) {
    assert.doesNotMatch(
      docsByPath[path],
      /tag push/i,
      `${path} still describes tag-push stable publication`,
    );
  }
  assert.doesNotMatch(changelog, /## \[Unreleased\]/);
});
