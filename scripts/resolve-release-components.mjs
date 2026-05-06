import { execFileSync } from "node:child_process";
import { loadReleaseComponents } from "./release-components.mjs";
import { sortStrings } from "./release-scope-utils.mjs";

function parseManualChangedFiles(value) {
  if (!value) {
    return [];
  }

  return value
    .split(/\r?\n/)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function collectChangedFilesFromGit(beforeSha, currentSha) {
  if (!beforeSha || !currentSha) {
    throw new Error("BEFORE_SHA and CURRENT_SHA are required when MANUAL_CHANGED_FILES is not provided");
  }

  const output = execFileSync("git", ["diff", "--name-only", beforeSha, currentSha], {
    encoding: "utf8",
  });

  return output
    .split(/\r?\n/)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function classifyPaths(changedFiles, graph) {
  const directComponents = new Set();
  const nonReleasePaths = [];
  const unmappedPaths = [];
  const reasons = {};

  function addReason(componentId, reason) {
    if (!reasons[componentId]) {
      reasons[componentId] = [];
    }
    if (!reasons[componentId].includes(reason)) {
      reasons[componentId].push(reason);
    }
  }

  for (const changedFile of changedFiles) {
    const sharedInfraComponents = graph.sharedInfraPaths[changedFile];
    if (sharedInfraComponents) {
      for (const componentId of sharedInfraComponents) {
        directComponents.add(componentId);
        addReason(componentId, `shared-infra:${changedFile}`);
      }
      continue;
    }

    const ownedComponentIds = Object.entries(graph.components)
      .filter(([, component]) => component.ownedPaths.some((ownedPath) => changedFile.startsWith(ownedPath)))
      .map(([componentId]) => componentId);

    if (ownedComponentIds.length > 0) {
      for (const componentId of ownedComponentIds) {
        directComponents.add(componentId);
        addReason(componentId, `owned:${changedFile}`);
      }
      continue;
    }

    if (graph.nonReleasePaths.some((nonReleasePath) => changedFile === nonReleasePath || changedFile.startsWith(nonReleasePath))) {
      nonReleasePaths.push(changedFile);
      continue;
    }

    unmappedPaths.push(changedFile);
  }

  return { directComponents, nonReleasePaths, unmappedPaths, reasons };
}

function ensureReasonBucket(reasons, componentId) {
  if (!reasons[componentId]) {
    reasons[componentId] = [];
  }

  return reasons[componentId];
}

function addTransitiveComponent(affectedComponents, reasons, componentId, dependency) {
  if (!affectedComponents.has(dependency) || affectedComponents.has(componentId)) {
    return false;
  }

  affectedComponents.add(componentId);
  ensureReasonBucket(reasons, componentId).push(`depends-on-release-of:${dependency}`);
  return true;
}

function expandTransitiveComponents(directComponents, graph, reasons) {
  const affectedComponents = new Set(directComponents);
  let changed = true;

  while (changed) {
    changed = false;

    for (const [componentId, component] of Object.entries(graph.components)) {
      for (const dependency of component.dependsOnReleaseOf) {
        changed = addTransitiveComponent(affectedComponents, reasons, componentId, dependency) || changed;
      }
    }
  }

  return affectedComponents;
}

export function resolveReleaseComponents({
  eventName = process.env.EVENT_NAME,
  beforeSha = process.env.BEFORE_SHA,
  currentSha = process.env.CURRENT_SHA,
  manualChangedFiles = parseManualChangedFiles(process.env.MANUAL_CHANGED_FILES),
  strict = process.env.STRICT_RELEASE_GRAPH === "true",
} = {}) {
  const graph = loadReleaseComponents();
  const changedFiles = manualChangedFiles.length > 0
    ? manualChangedFiles
    : collectChangedFilesFromGit(beforeSha, currentSha);

  const { directComponents, nonReleasePaths, unmappedPaths, reasons } = classifyPaths(changedFiles, graph);
  const affectedComponents = expandTransitiveComponents(directComponents, graph, reasons);
  const sortedAffectedComponents = sortStrings([...affectedComponents]);
  const sortedDirectComponents = sortStrings([...directComponents]);
  const sortedTransitiveComponents = sortStrings(
    sortedAffectedComponents.filter((componentId) => !directComponents.has(componentId)),
  );

  const result = {
    event_name: eventName ?? null,
    changed_files: changedFiles,
    affected_components: sortedAffectedComponents,
    direct_components: sortedDirectComponents,
    transitive_components: sortedTransitiveComponents,
    non_release_paths: sortStrings(nonReleasePaths),
    unmapped_paths: sortStrings(unmappedPaths),
    reasons,
  };

  if (strict && result.unmapped_paths.length > 0) {
    const error = new Error(`Unmapped release-relevant paths: ${result.unmapped_paths.join(", ")}`);
    error.result = result;
    throw error;
  }

  return result;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  try {
    const resolved = resolveReleaseComponents();
    process.stdout.write(`${JSON.stringify(resolved, null, 2)}\n`);
  } catch (error) {
    if (error.result) {
      process.stdout.write(`${JSON.stringify(error.result, null, 2)}\n`);
    }
    process.stderr.write(`${error.message}\n`);
    process.exit(1);
  }
}
