#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const argv = process.argv.slice(2);
const wantsHelp = argv.includes("--help");
const wantsWrite = argv.includes("--write");
const wantsCheck = argv.includes("--check");
const manifestPath = resolveOption("--manifest", ".release-please-manifest.json");
const mode = resolveMode(wantsWrite, wantsCheck);

const publishableNpmSurfaces = {
  "clients/agent-runtime": {
    packageName: "@dallay/corvus",
    packages: [
      "clients/agent-runtime/npm/corvus/package.json",
      "clients/agent-runtime/npm/corvus-darwin-x64/package.json",
      "clients/agent-runtime/npm/corvus-darwin-arm64/package.json",
      "clients/agent-runtime/npm/corvus-linux-x64/package.json",
      "clients/agent-runtime/npm/corvus-linux-arm64/package.json",
      "clients/agent-runtime/npm/corvus-windows-x64/package.json",
    ],
  },
  "clients/rook": {
    packageName: "@dallay/rook",
    packages: [
      "clients/rook/npm/rook/package.json",
      "clients/rook/npm/rook-darwin-x64/package.json",
      "clients/rook/npm/rook-darwin-arm64/package.json",
      "clients/rook/npm/rook-linux-x64/package.json",
      "clients/rook/npm/rook-linux-arm64/package.json",
      "clients/rook/npm/rook-windows-x64/package.json",
    ],
  },
};

if (wantsHelp) {
  process.stdout.write("Usage: node scripts/sync-release-npm-manifests.mjs [--check|--write] [--manifest <path>]\n");
  process.stdout.write("  --check             Validate publishable npm package versions against release-please manifest\n");
  process.stdout.write("  --write             Rewrite stale publishable npm package versions\n");
  process.stdout.write("  --manifest <path>   Release Please manifest path (default: .release-please-manifest.json)\n");
  process.exit(0);
}

if (!mode) {
  throw new Error("Exactly one of --check or --write must be provided");
}

function resolveMode(wantsWrite, wantsCheck) {
  if (wantsWrite === wantsCheck) {
    return null;
  }

  return wantsWrite ? "write" : "check";
}

function resolveOption(name, defaultValue) {
  const optionIndex = argv.indexOf(name);
  if (optionIndex === -1) {
    return defaultValue;
  }

  const value = argv[optionIndex + 1];
  if (!value || value.startsWith("--")) {
    throw new Error(`${name} requires a value`);
  }

  return value;
}

function resolvePath(relativePath) {
  return path.resolve(repoRoot, relativePath);
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(resolvePath(relativePath), "utf8"));
}

function writeJson(relativePath, data) {
  fs.writeFileSync(resolvePath(relativePath), `${JSON.stringify(data, null, 2)}\n`, "utf8");
}

function expectedOptionalDependencyNames(componentConfig) {
  return componentConfig.packages
    .slice(1)
    .map((packagePath) => readJson(packagePath).name);
}

function syncPackageManifest(packagePath, expectedVersion, expectedOptionalDependencySet) {
  const manifest = readJson(packagePath);
  const changes = [];

  if (manifest.version !== expectedVersion) {
    changes.push(`${packagePath} version ${manifest.version} -> ${expectedVersion}`);
    manifest.version = expectedVersion;
  }

  if (manifest.optionalDependencies && expectedOptionalDependencySet.size > 0) {
    for (const dependencyName of expectedOptionalDependencySet) {
      if (!Object.hasOwn(manifest.optionalDependencies, dependencyName)) {
        throw new Error(`missing-optional-dependency: ${packagePath} does not declare ${dependencyName}`);
      }

      const actualVersion = manifest.optionalDependencies[dependencyName];
      if (actualVersion !== expectedVersion) {
        changes.push(`${packagePath} optionalDependencies.${dependencyName} ${actualVersion} -> ${expectedVersion}`);
        manifest.optionalDependencies[dependencyName] = expectedVersion;
      }
    }
  }

  if (changes.length > 0 && mode === "write") {
    writeJson(packagePath, manifest);
  }

  return changes;
}

const releaseManifest = readJson(manifestPath);
const changes = [];
const checkedComponents = [];

for (const [componentPath, componentConfig] of Object.entries(publishableNpmSurfaces)) {
  const expectedVersion = releaseManifest[componentPath];
  if (!expectedVersion) {
    continue;
  }

  checkedComponents.push(`${componentConfig.packageName}@${expectedVersion}`);
  const optionalDependencyNames = expectedOptionalDependencyNames(componentConfig);
  const expectedOptionalDependencySet = new Set(optionalDependencyNames);

  for (const packagePath of componentConfig.packages) {
    changes.push(...syncPackageManifest(packagePath, expectedVersion, expectedOptionalDependencySet));
  }
}

if (checkedComponents.length === 0) {
  process.stdout.write(`No publishable npm surfaces found in ${manifestPath}\n`);
  process.exit(0);
}

for (const checkedComponent of checkedComponents) {
  process.stdout.write(`Checked ${checkedComponent}\n`);
}

if (changes.length === 0) {
  process.stdout.write("All release npm manifests are aligned\n");
  process.exit(0);
}

for (const change of changes) {
  process.stdout.write(`${change}\n`);
}

if (mode === "check") {
  throw new Error(`release-npm-version-drift: ${changes.length} npm manifest update(s) required`);
}

process.stdout.write("Release npm manifest updates were written successfully\n");
