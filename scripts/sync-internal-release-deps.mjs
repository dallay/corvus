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
  for (const line of manifestText.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed.startsWith(`${dependencyName} = {`) || !trimmed.endsWith("}")) {
      continue;
    }
    return trimmed;
  }

  throw new Error(`missing-dependency-entry: ${dependencyName} not found in ${manifestPath}`);
}

function extractField(block, fieldName, manifestPath, dependencyName) {
  const fields = block
    .slice(block.indexOf("{") + 1, block.lastIndexOf("}"))
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);

  for (const entry of fields) {
    const [rawKey, rawValue] = entry.split("=").map((part) => part.trim());
    if (rawKey !== fieldName || !rawValue?.startsWith('"') || !rawValue.endsWith('"')) {
      continue;
    }
    return rawValue.slice(1, -1);
  }

  throw new Error(`Could not resolve ${fieldName} for ${dependencyName} in ${manifestPath}`);
}

function updateVersionInBlock(block, expectedVersion) {
  return block.replace(/version\s*=\s*"([^"]+)"/, `version = "${expectedVersion}"`);
}

function resolveVersionBySelector(manifestText, manifestPath, versionSelector) {
  if (versionSelector === "package.version") {
    return extractPackageVersion(manifestText, manifestPath);
  }

  const parts = versionSelector.split(".");
  if (parts.length !== 3 || parts[0] !== "dependencies" || parts[2] !== "version") {
    throw new Error(`upstream-version-unresolvable: unsupported versionSelector ${versionSelector} for ${manifestPath}`);
  }

  const dependencyName = parts[1];
  const dependencyBlock = extractDependencyBlock(manifestText, dependencyName, manifestPath);
  return extractField(dependencyBlock, "version", manifestPath, dependencyName);
}

function collectInternalPathDependencies(manifestText) {
  const dependencies = [];
  const dependencyRegex = /^([A-Za-z0-9_-]+)\s*=\s*\{([^\n]*path\s*=\s*"([^\"]+)"[^\n]*)\}$/gm;

  for (const match of manifestText.matchAll(dependencyRegex)) {
    if (!match[2].includes("version")) {
      continue;
    }

    dependencies.push({
      dependencyName: match[1],
      path: match[3],
      block: match[0],
    });
  }

  return dependencies;
}

const graph = loadReleaseComponents();
const changes = [];
let rewrites = 0;

const edgeKey = (manifestPath, dependencyName) => `${manifestPath}::${dependencyName}`;
const managedEdges = new Map(graph.internalReleaseDependencies.map((edge) => [edgeKey(edge.manifestPath, edge.dependencyName), edge]));
const componentEntries = Object.entries(graph.components);

function resolveManifestRelativeClientTarget(manifestPath, dependencyPath) {
  const manifestDirectory = path.posix.dirname(manifestPath);
  return path.posix.normalize(path.posix.join(manifestDirectory, dependencyPath));
}

function findOwningComponentId(targetPath) {
  for (const [componentId, component] of componentEntries) {
    if (component.ownedPaths.some((ownedPath) => targetPath === ownedPath.slice(0, -1))) {
      return componentId;
    }
  }

  return null;
}

const manifestTexts = new Map();
const releaseManagedManifestPaths = new Set([
  ...graph.internalReleaseDependencies.map((edge) => edge.manifestPath),
  ...Object.values(graph.components)
    .flatMap((component) => component.versionSurfaces)
    .filter((surface) => surface.endsWith("Cargo.toml") && surface.startsWith("clients/")),
]);

for (const manifestPath of releaseManagedManifestPaths) {
  manifestTexts.set(manifestPath, readText(manifestPath));
}

for (const [manifestPath, manifestText] of manifestTexts.entries()) {
  const manifestOwner = findOwningComponentId(manifestPath);

  for (const dependency of collectInternalPathDependencies(manifestText)) {
    const resolvedDependencyTarget = resolveManifestRelativeClientTarget(manifestPath, dependency.path);
    const targetOwner = findOwningComponentId(resolvedDependencyTarget);
    if (!targetOwner || targetOwner === manifestOwner) {
      continue;
    }

    if (!managedEdges.has(edgeKey(manifestPath, dependency.dependencyName))) {
      const message = `unmanaged-internal-release-edge: ${manifestPath} declares ${dependency.dependencyName} -> ${dependency.path} without internalReleaseDependencies coverage`;
      changes.push(message);
      throw new Error(message);
    }
  }
}

for (const edge of graph.internalReleaseDependencies) {
  const upstreamManifestPath = graph.components[edge.upstreamComponent].versionSurfaces.find((entry) => entry.endsWith("Cargo.toml"));
  if (!upstreamManifestPath) {
    throw new Error(`upstream-version-unresolvable: no Cargo.toml version surface found for ${edge.upstreamComponent}`);
  }

  const upstreamText = readText(upstreamManifestPath);
  const downstreamText = readText(edge.manifestPath);
  const expectedVersion = resolveVersionBySelector(upstreamText, upstreamManifestPath, edge.versionSelector);
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
