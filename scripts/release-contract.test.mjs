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
const releaseVersion = readText("version.txt").trim();

const contractDocs = [
  ".github/workflows/README.md",
  "clients/web/apps/docs/src/content/docs/guides/release.md",
  "clients/web/apps/docs/src/content/docs/es/guides/release.md",
  "clients/web/apps/docs/src/content/docs/clients/agent-runtime/ci-map.md",
  "clients/web/apps/docs/src/content/docs/es/clients/agent-runtime/ci-map.md",
  "CHANGELOG.md",
];

test("release-please fan-out only includes shipped stable artifacts", () => {
  const config = readJson("release-please-config.json");
  const extraFiles = config.packages["."]["extra-files"];
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
  assert.ok(!filePaths.has("clients/web/**/package.json"));
  assert.ok(!filePaths.has("clients/agent-runtime/npm/**/package.json"));
  assert.ok(!filePaths.has("clients/agent-runtime/npm/corvus-cli/package.json"));
  assert.ok(!filePaths.has("clients/agent-runtime/npm/corvus-windows-arm64/package.json"));

  for (const expectedPath of [
    "gradle.properties",
    "gradle/build-logic/gradle.properties",
    "clients/agent-runtime/Cargo.toml",
    "modules/cerebro/Cargo.toml",
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
  assert.ok(cargoTomlTargets.has("$.dependencies.cerebro.version"));
});

test("beta release-please config reuses shipped artifact fan-out with prerelease semantics", () => {
  const stableConfig = readJson("release-please-config.json");
  const betaConfig = readJson("release-please-beta-config.json");
  const stablePackage = stableConfig.packages["."];
  const betaPackage = betaConfig.packages["."];

  assert.deepEqual(betaPackage["extra-files"], stablePackage["extra-files"]);
  assert.equal(betaPackage["release-type"], stablePackage["release-type"]);
  assert.equal(betaPackage["version-file"], stablePackage["version-file"]);
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

test("sortStrings uses an explicit stable comparator", () => {
  assert.deepEqual(sortStrings(["pkg-2", "pkg-10", "pkg-1"]), [
    "pkg-1",
    "pkg-2",
    "pkg-10",
  ]);
});

test("release workflows encode release-please-owned stable and beta governance", () => {
  const releasePlease = readText(".github/workflows/release-please.yml");
  const releasePleaseBeta = readText(".github/workflows/release-please-beta.yml");
  const publishRelease = readText(".github/workflows/publish-release.yml");
  const publishSnapshot = readText(".github/workflows/publish-snapshot.yml");
  const publishWorkflow = readText(".github/workflows/_publish.yml");

  assertIncludesAll(
    releasePlease,
    [
      /id: release-please/,
      /manifest-file: \.release-please-manifest\.json/,
      /release-please action outputs/i,
      /canonical GitHub Release/i,
      /release PR\/tag\/GitHub Release path/i,
    ],
    "release-please workflow",
  );
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
    ],
    "release-please-beta workflow",
  );
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
    ],
    "publish-release workflow",
  );
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
      /gh release upload/,
      /release-please owns canonical stable release notes/i,
      /release-please owns canonical beta release notes/i,
      /Existing GitHub Release asset upload/i,
      /corvus-cli is internal\/private/,
      /Windows ARM64 is intentionally unsupported/,
    ],
    "_publish workflow",
  );
  assert.doesNotMatch(publishWorkflow, /release-changelog-builder-action/);
  assert.doesNotMatch(publishWorkflow, /softprops\/action-gh-release/);
  assert.doesNotMatch(publishWorkflow, /\.github\/config\/changelog\.json/);
  assert.doesNotMatch(publishWorkflow, /inputs\.changelog/);
});

test("cargo publish contract keeps local cerebro path and release version aligned", () => {
  const cargoToml = readText("clients/agent-runtime/Cargo.toml");
  const cerebroToml = readText("modules/cerebro/Cargo.toml");

  assert.match(
    cargoToml,
    new RegExp(
      `cerebro = \\{ version = "${escapeRegex(releaseVersion)}", path = "\\.\\.\\/\\.\\.\\/modules\\/cerebro" \\}`,
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

  for (const cwd of ["clients/agent-runtime", "modules/cerebro"]) {
    try {
      execFileSync(cargoExecutable, ["metadata", "--locked", "--format-version", "1"], {
        cwd,
        stdio: "pipe",
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
  const changelog = docsByPath["CHANGELOG.md"];

  assertIncludesAll(
    docsEn,
    [
      /release-please-beta\.yml/i,
      /beta branch/i,
      /beta releases use the npm `beta` dist-tag/i,
      /private web packages are excluded/i,
      /manual recovery/i,
      /through a pull request/i,
      /attach assets to the existing GitHub Release/i,
    ],
    "English release runbook",
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
    workflowsReadme,
    [
      /release-please .*canonical.*GitHub Release/i,
      /release-please-beta\.yml.*beta/i,
      /publish-release\.yml.*release\.published/i,
      /_publish\.yml.*attach artifacts/i,
    ],
    "workflow README",
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
