import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import test from "node:test";
import fs from "node:fs";

function readJson(path) {
  return JSON.parse(fs.readFileSync(path, "utf8"));
}

function readText(path) {
  return fs.readFileSync(path, "utf8");
}

function assertIncludesAll(text, patterns, label) {
  for (const pattern of patterns) {
    assert.match(text, pattern, `${label} is missing ${pattern}`);
  }
}

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
  const filePaths = extraFiles.map((entry) => entry.path);
  const cargoTomlTargets = extraFiles
    .filter((entry) => entry.path === "clients/agent-runtime/Cargo.toml")
    .map((entry) => entry.jsonpath);
  const optionalDependencyPins = extraFiles
    .filter((entry) => entry.path === "clients/agent-runtime/npm/corvus/package.json")
    .map((entry) => entry.jsonpath);

  assert.equal(config["bootstrap-sha"], undefined);
  assert.equal(config["skip-github-release"], undefined);
  assert.equal(config["skip-changelog"], undefined);
  assert.ok(!filePaths.includes("clients/web/**/package.json"));
  assert.ok(!filePaths.includes("clients/agent-runtime/npm/**/package.json"));
  assert.ok(!filePaths.includes("clients/agent-runtime/npm/corvus-cli/package.json"));
  assert.ok(!filePaths.includes("clients/agent-runtime/npm/corvus-windows-arm64/package.json"));

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
    assert.ok(filePaths.includes(expectedPath), `missing version target: ${expectedPath}`);
  }

  assert.ok(
    !optionalDependencyPins.includes(
      "$.optionalDependencies['@dallay/corvus-windows-arm64']",
    ),
  );
  assert.ok(cargoTomlTargets.includes("$.package.version"));
  assert.ok(cargoTomlTargets.includes("$.dependencies.cerebro.version"));
});

test("runtime npm metadata only advertises supported shipped platforms", () => {
  const pkg = readJson("clients/agent-runtime/npm/corvus/package.json");

  assert.deepEqual(Object.keys(pkg.optionalDependencies).sort(), [
    "@dallay/corvus-darwin-arm64",
    "@dallay/corvus-darwin-x64",
    "@dallay/corvus-linux-arm64",
    "@dallay/corvus-linux-x64",
    "@dallay/corvus-windows-x64",
  ]);
});

test("release workflows encode release-please-owned stable governance", () => {
  const releasePlease = readText(".github/workflows/release-please.yml");
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

  assertIncludesAll(
    publishRelease,
    [
      /on:\s*release:\s*types:\s*- published/s,
      /Canonical stable release handoff/i,
      /release: true/,
      /release_tag: \$\{\{ github\.event\.release\.tag_name \}\}/,
      /release_id: \$\{\{ github\.event\.release\.id \}\}/,
      /attach artifacts to the existing canonical GitHub Release/i,
    ],
    "publish-release workflow",
  );
  assert.doesNotMatch(publishRelease, /push:\s*tags:/s);
  assert.doesNotMatch(publishRelease, /changelog:\s*true/);

  assertIncludesAll(
    publishSnapshot,
    [
      /Snapshots stay outside stable GitHub Release ownership/,
      /release: false/,
    ],
    "publish-snapshot workflow",
  );
  assert.doesNotMatch(publishSnapshot, /changelog:/);

  assertIncludesAll(
    publishWorkflow,
    [
      /release_tag:/,
      /release_id:/,
      /release_version/,
      /gh release upload/,
      /release-please owns canonical stable release notes/i,
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

  assert.match(
    cargoToml,
    /cerebro = \{ version = "1\.0\.0", path = "\.\.\/\.\.\/modules\/cerebro" \}/,
  );
});

test("rust lockfiles stay valid for --locked release commands", () => {
  for (const cwd of ["clients/agent-runtime", "modules/cerebro"]) {
    execFileSync("cargo", ["metadata", "--locked", "--format-version", "1"], {
      cwd,
      stdio: "ignore",
    });
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
