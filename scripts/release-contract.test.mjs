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

test("release workflows document canonical ownership and diagnostics", () => {
  const releasePlease = readText(".github/workflows/release-please.yml");
  const publishRelease = readText(".github/workflows/publish-release.yml");
  const publishSnapshot = readText(".github/workflows/publish-snapshot.yml");
  const publishWorkflow = readText(".github/workflows/_publish.yml");

  assert.match(releasePlease, /id: release-please/);
  assert.match(releasePlease, /Checkout release metadata/);
  assert.match(releasePlease, /GITHUB_STEP_SUMMARY/);
  assert.match(releasePlease, /manifest-file: \.release-please-manifest\.json/);
  assert.match(releasePlease, /release-please action outputs/i);

  assert.match(publishRelease, /Canonical stable release handoff/);
  assert.match(publishRelease, /release: true/);
  assert.match(publishRelease, /changelog: true/);

  assert.match(publishSnapshot, /Snapshots stay outside stable GitHub Release ownership/);
  assert.match(publishSnapshot, /release: false/);
  assert.match(publishSnapshot, /changelog: false/);

  assert.doesNotMatch(publishWorkflow, /clients\/web\/apps\/\*\/package\.json/);
  assert.match(publishWorkflow, /version\.txt/);
  assert.match(publishWorkflow, /corvus-windows-x64\/package\.json/);
  assert.match(publishWorkflow, /gradle\/build-logic publishToMavenCentral/);
  assert.doesNotMatch(publishWorkflow, /\.\/gradlew publishToMavenCentral/);
  assert.match(publishWorkflow, /corvus-cli is internal\/private/);
  assert.match(publishWorkflow, /Windows ARM64 is intentionally unsupported/);
  assert.match(publishWorkflow, /Release publish summary/);
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

test("release docs and changelog point to GitHub Releases as canonical notes", () => {
  const docsEn = readText("clients/web/apps/docs/src/content/docs/guides/release.md");
  const docsEs = readText("clients/web/apps/docs/src/content/docs/es/guides/release.md");
  const workflowsReadme = readText(".github/workflows/README.md");
  const changelog = readText("CHANGELOG.md");

  for (const doc of [docsEn, docsEs, workflowsReadme]) {
    assert.match(doc, /GitHub Release/);
    assert.match(doc, /release-please/i);
    assert.match(doc, /corvus-cli/);
    assert.match(doc, /Windows ARM64/);
  }

  assert.match(docsEn, /private web packages are excluded/i);
  assert.match(docsEn, /manual recovery/i);
  assert.match(docsEn, /through a pull request/i);
  assert.match(docsEs, /paquetes web privados están excluidos/i);
  assert.match(docsEs, /recuperación manual/i);
  assert.match(changelog, /GitHub Releases/);
  assert.doesNotMatch(changelog, /## \[Unreleased\]/);
});
