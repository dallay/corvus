import fs from "node:fs";
import path from "node:path";

const RELEASE_COMPONENTS_PATH = path.resolve("config/release-components.json");
const VALID_PUBLISH_POLICIES = new Set(["publishable", "validate-only"]);

function readReleaseComponentsFile() {
  return JSON.parse(fs.readFileSync(RELEASE_COMPONENTS_PATH, "utf8"));
}

function validateStringArray(value, label) {
  if (!Array.isArray(value) || value.some((entry) => typeof entry !== "string" || entry.length === 0)) {
    throw new Error(`${label} must be a non-empty string array when provided`);
  }
}

function validateGraph(graph) {
  if (!graph || typeof graph !== "object" || Array.isArray(graph)) {
    throw new Error("release component graph must be an object");
  }

  if (!graph.components || typeof graph.components !== "object" || Array.isArray(graph.components)) {
    throw new Error("release component graph must define a components object");
  }

  if (!graph.sharedInfraPaths || typeof graph.sharedInfraPaths !== "object" || Array.isArray(graph.sharedInfraPaths)) {
    throw new Error("release component graph must define a sharedInfraPaths object");
  }

  validateStringArray(graph.nonReleasePaths, "nonReleasePaths");

  const componentIds = Object.keys(graph.components);
  if (componentIds.length === 0) {
    throw new Error("release component graph must define at least one component");
  }

  for (const componentId of componentIds) {
    const component = graph.components[componentId];
    if (!component || typeof component !== "object" || Array.isArray(component)) {
      throw new Error(`component ${componentId} must be an object`);
    }

    if (!VALID_PUBLISH_POLICIES.has(component.publishPolicy)) {
      throw new Error(`component ${componentId} has unsupported publishPolicy ${component.publishPolicy}`);
    }

    validateStringArray(component.ownedPaths, `${componentId}.ownedPaths`);
    validateStringArray(component.versionSurfaces, `${componentId}.versionSurfaces`);
    validateStringArray(component.releaseChannels, `${componentId}.releaseChannels`);

    if (!Array.isArray(component.dependsOnReleaseOf)) {
      throw new Error(`component ${componentId}.dependsOnReleaseOf must be an array`);
    }

    for (const dependency of component.dependsOnReleaseOf) {
      if (typeof dependency !== "string" || dependency.length === 0) {
        throw new Error(`component ${componentId} has invalid dependency entry`);
      }
      if (!graph.components[dependency]) {
        throw new Error(`component ${componentId} depends on unknown component ${dependency}`);
      }
    }
  }

  for (const [sharedPath, componentIdsForPath] of Object.entries(graph.sharedInfraPaths)) {
    if (typeof sharedPath !== "string" || sharedPath.length === 0) {
      throw new Error("sharedInfraPaths keys must be non-empty strings");
    }
    validateStringArray(componentIdsForPath, `sharedInfraPaths.${sharedPath}`);
    for (const componentId of componentIdsForPath) {
      if (!graph.components[componentId]) {
        throw new Error(`sharedInfraPaths.${sharedPath} references unknown component ${componentId}`);
      }
    }
  }

  return graph;
}

export function loadReleaseComponents() {
  return validateGraph(readReleaseComponentsFile());
}

export { RELEASE_COMPONENTS_PATH, VALID_PUBLISH_POLICIES };
