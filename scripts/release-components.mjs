import fs from "node:fs";
import path from "node:path";

const RELEASE_COMPONENTS_PATH = path.resolve("config/release-components.json");
const VALID_PUBLISH_POLICIES = new Set(["publishable", "validate-only"]);
const VALID_INTERNAL_RELEASE_DEPENDENCY_MODES = new Set(["must-match-release-version"]);
const REQUIRED_INTERNAL_DEPENDENCY_FIELDS = [
  "dependentComponent",
  "upstreamComponent",
  "manifestPath",
  "dependencyName",
  "dependencyPath",
  "versionSelector",
  "mode",
  "notes",
];

function readReleaseComponentsFile() {
  return JSON.parse(fs.readFileSync(RELEASE_COMPONENTS_PATH, "utf8"));
}

function validateStringArray(value, label) {
  if (!Array.isArray(value) || value.some((entry) => typeof entry !== "string" || entry.length === 0)) {
    throw new Error(`${label} must be a non-empty string array when provided`);
  }
}

function validateInternalReleaseDependencyShape(edge, index) {
  if (!edge || typeof edge !== "object" || Array.isArray(edge)) {
    throw new Error(`internalReleaseDependencies[${index}] must be an object`);
  }

  for (const field of REQUIRED_INTERNAL_DEPENDENCY_FIELDS) {
    if (typeof edge[field] !== "string" || edge[field].length === 0) {
      throw new Error(`internalReleaseDependencies[${index}].${field} must be a non-empty string`);
    }
  }
}

function validateInternalReleaseDependencyReferences(graph, edge, index) {
  if (!graph.components[edge.dependentComponent]) {
    throw new Error(
      `internalReleaseDependencies[${index}] references unknown dependent component ${edge.dependentComponent}`,
    );
  }

  if (!graph.components[edge.upstreamComponent]) {
    throw new Error(
      `internalReleaseDependencies[${index}] references unknown upstream component ${edge.upstreamComponent}`,
    );
  }

  if (!VALID_INTERNAL_RELEASE_DEPENDENCY_MODES.has(edge.mode)) {
    throw new Error(
      `internalReleaseDependencies[${index}] has unsupported mode ${edge.mode}`,
    );
  }
}

function validateInternalReleaseDependencies(graph) {
  if (!Array.isArray(graph.internalReleaseDependencies)) {
    throw new Error("release component graph must define an internalReleaseDependencies array");
  }

  for (const [index, edge] of graph.internalReleaseDependencies.entries()) {
    validateInternalReleaseDependencyShape(edge, index);
    validateInternalReleaseDependencyReferences(graph, edge, index);
  }
}

function validateGraphShape(graph) {
  if (!graph || typeof graph !== "object" || Array.isArray(graph)) {
    throw new TypeError("release component graph must be an object");
  }

  if (!graph.components || typeof graph.components !== "object" || Array.isArray(graph.components)) {
    throw new Error("release component graph must define a components object");
  }

  if (!graph.sharedInfraPaths || typeof graph.sharedInfraPaths !== "object" || Array.isArray(graph.sharedInfraPaths)) {
    throw new Error("release component graph must define a sharedInfraPaths object");
  }
}

function validateComponent(graph, componentId) {
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

function validateSharedInfraPath(graph, sharedPath, componentIdsForPath) {
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

function validateGraph(graph) {
  validateGraphShape(graph);
  validateStringArray(graph.nonReleasePaths, "nonReleasePaths");
  validateInternalReleaseDependencies(graph);

  const componentIds = Object.keys(graph.components);
  if (componentIds.length === 0) {
    throw new Error("release component graph must define at least one component");
  }

  for (const componentId of componentIds) {
    validateComponent(graph, componentId);
  }

  for (const [sharedPath, componentIdsForPath] of Object.entries(graph.sharedInfraPaths)) {
    validateSharedInfraPath(graph, sharedPath, componentIdsForPath);
  }

  return graph;
}

export function loadReleaseComponents() {
  return validateGraph(readReleaseComponentsFile());
}

export {
  RELEASE_COMPONENTS_PATH,
  VALID_PUBLISH_POLICIES,
  VALID_INTERNAL_RELEASE_DEPENDENCY_MODES,
};
