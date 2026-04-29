#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { loadReleaseComponents } from "./release-components.mjs";

const repoRoot = process.cwd();
const argv = process.argv.slice(2);
const wantsHelp = argv.includes("--help");
const mode = argv.includes("--write") ? "write" : argv.includes("--check") ? "check" : null;

if (wantsHelp) {
  process.stdout.write("Usage: node scripts/sync-internal-release-deps.mjs [--check|--write]\n");
  process.stdout.write("  --check  Validate internal release dependency pins\n");
  process.stdout.write("  --write  Rewrite stale internal release dependency pins\n");
  process.exit(0);
}

if (!mode || (argv.includes("--check") && argv.includes("--write"))) {
  throw new Error("Exactly one of --check or --write must be provided");
}

function resolvePath(relativePath) {
  return path.resolve(repoRoot, relativePath);
}

function readText(relativePath) {
  return fs.readFileSync(resolvePath(relativePath), "utf8");
}

function writeText(relativePath, content) {
  fs.writeFileSync(resolvePath(relativePath), content, "utf8");
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function extractPackageVersion(manifestText, manifestPath) {
  const packageBlockMatch = manifestText.match(/\[package\]([\s\S]*?)(\n\[|$)/);
  if (!packageBlockMatch) {
    throw new Error(`Could not find [package] block in ${manifestPath}`);
  }

  const versionMatch = packageBlockMatch[1].match(/^version\s*=\s*"([^"]+)"$/m);
  if (!versionMatch) {
    throw new Error(`Could not resolve package.version from ${manifestPath}`);
  }

  return versionMatch[1];
}

function extractDependencyBlock(manifestText, dependencyName, manifestPath) {
  const regex = new RegExp(`^(${escapeRegex(dependencyName)}\\s*=\\s*\\{[^\\n]*\\})$`, "m");
  const match = manifestText.match(regex);
  if (!match) {
    throw new Error(`missing-dependency-entry: ${dependencyName} not found in ${manifestPath}`);
  }
  return match[1];
}

function extractField(block, fieldName, manifestPath, dependencyName) {
  const match = block.match(new RegExp(`${escapeRegex(fieldName)}\\s*=\\s*"([^"]+)"`));
  if (!match) {
    throw new Error(`Could not resolve ${fieldName} for ${dependencyName} in ${manifestPath}`);
  }
  return match[1];
}

function updateVersionInBlock(block, expectedVersion) {
  return block.replace(/version\s*=\s*"([^"]+)"/, `version = "${expectedVersion}"`);
}

const graph = loadReleaseComponents();
const changes = [];
let rewrites = 0;

for (const edge of graph.internalReleaseDependencies) {
  const upstreamManifestPath = graph.components[edge.upstreamComponent].versionSurfaces.find((entry) => entry.endsWith("Cargo.toml"));
  if (!upstreamManifestPath) {
    throw new Error(`upstream-version-unresolvable: no Cargo.toml version surface found for ${edge.upstreamComponent}`);
  }

  const upstreamText = readText(upstreamManifestPath);
  const downstreamText = readText(edge.manifestPath);
  const expectedVersion = extractPackageVersion(upstreamText, upstreamManifestPath);
  const dependencyBlock = extractDependencyBlock(downstreamText, edge.dependencyName, edge.manifestPath);
  const actualVersion = extractField(dependencyBlock, "version", edge.manifestPath, edge.dependencyName);
  const actualPath = extractField(dependencyBlock, "path", edge.manifestPath, edge.dependencyName);

  if (actualPath !== edge.dependencyPath) {
    throw new Error(
      `path-mismatch: ${edge.manifestPath} ${edge.dependencyName} path expected ${edge.dependencyPath} but found ${actualPath}`,
    );
  }

  if (actualVersion !== expectedVersion) {
    if (mode === "check") {
      throw new Error(
        `version-drift: ${edge.dependentComponent} -> ${edge.upstreamComponent} expected ${expectedVersion} but found ${actualVersion} in ${edge.manifestPath}`,
      );
    }

    const updatedBlock = updateVersionInBlock(dependencyBlock, expectedVersion);
    writeText(edge.manifestPath, downstreamText.replace(dependencyBlock, updatedBlock));
    changes.push(`${edge.dependentComponent} -> ${edge.upstreamComponent}: ${actualVersion} -> ${expectedVersion}`);
    rewrites += 1;
    continue;
  }

  changes.push(`${edge.dependentComponent} -> ${edge.upstreamComponent}: already aligned at ${expectedVersion}`);
}

for (const line of changes) {
  process.stdout.write(`${line}\n`);
}

if (mode === "write") {
  if (rewrites === 0) {
    process.stdout.write("No internal release dependency updates were required\n");
  } else {
    process.stdout.write("Internal release dependency updates were written successfully\n");
  }
} else {
  process.stdout.write("All internal release dependencies are aligned\n");
}
