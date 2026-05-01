import { loadReleaseComponents } from "./release-components.mjs";
import { getPublishableComponentIds, sortStrings } from "./release-scope-utils.mjs";

function parseBoolean(value, name) {
  if (value === "true") {
    return true;
  }
  if (value === "false") {
    return false;
  }
  throw new Error(`${name} must be either "true" or "false"`);
}

function parseAffectedComponentsOverride(affectedComponentsRaw, graph) {
  let parsed;
  try {
    parsed = JSON.parse(affectedComponentsRaw);
  } catch (error) {
    throw new Error(`Invalid AFFECTED_COMPONENTS payload: ${affectedComponentsRaw}`, { cause: error });
  }

  if (!Array.isArray(parsed)) {
    throw new Error(`Invalid AFFECTED_COMPONENTS payload: ${affectedComponentsRaw}`);
  }

  const knownComponents = new Set(Object.keys(graph.components));
  const unknownComponents = sortStrings(parsed.filter((componentId) => !knownComponents.has(componentId)));
  if (unknownComponents.length > 0) {
    throw new Error(`Unknown affected components in _publish input: ${unknownComponents.join(", ")}`);
  }

  return sortStrings([...new Set(parsed)]);
}

function parseComponentReleaseTag(releaseTag, publishableComponents) {
  for (const componentId of publishableComponents) {
    const escapedComponentId = componentId.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const match = new RegExp(`^${escapedComponentId}-v([0-9]+)\\.([0-9]+)\\.([0-9]+)(?:-beta\\.([0-9]+))?$`).exec(
      releaseTag,
    );

    if (match) {
      const [, major, minor, patch, betaNumber] = match;
      return {
        componentId,
        major,
        minor,
        patch,
        betaNumber,
        version: betaNumber ? `${major}.${minor}.${patch}-beta.${betaNumber}` : `${major}.${minor}.${patch}`,
        channel: betaNumber ? "beta" : "stable",
      };
    }
  }

  throw new Error(`Unsupported component-scoped release tag: ${releaseTag}`);
}

export function resolveReleaseContext({
  releaseTag = process.env.RELEASE_TAG,
  releaseId = process.env.RELEASE_ID ?? "",
  prerelease = process.env.PRERELEASE ?? "false",
  affectedComponentsRaw = process.env.AFFECTED_COMPONENTS ?? "[]",
} = {}) {
  if (!releaseTag) {
    throw new Error("RELEASE_TAG is required");
  }

  const graph = loadReleaseComponents();
  const publishableComponents = getPublishableComponentIds(graph);
  const parsedTag = parseComponentReleaseTag(releaseTag, publishableComponents);
  const isPrerelease = parseBoolean(prerelease, "PRERELEASE");

  if (parsedTag.channel === "stable" && isPrerelease) {
    throw new Error(`Stable release tag ${releaseTag} cannot be published from a prerelease GitHub Release`);
  }

  if (parsedTag.channel === "beta" && !isPrerelease) {
    throw new Error(`Beta release tag ${releaseTag} requires a prerelease GitHub Release`);
  }

  const affectedComponentsOverride = parseAffectedComponentsOverride(affectedComponentsRaw, graph);
  const effectiveAffectedComponents = affectedComponentsOverride.length > 0 ? affectedComponentsOverride : [parsedTag.componentId];

  if (effectiveAffectedComponents.length !== 1 || effectiveAffectedComponents[0] !== parsedTag.componentId) {
    throw new Error(
      `AFFECTED_COMPONENTS must match release tag component exactly: expected ${parsedTag.componentId}, got ${effectiveAffectedComponents.join(", ")}`,
    );
  }

  return {
    release_tag: releaseTag,
    release_id: releaseId,
    release_component: parsedTag.componentId,
    release_version: parsedTag.version,
    release_channel: parsedTag.channel,
    release_major_minor: `${parsedTag.major}.${parsedTag.minor}`,
    release_major: parsedTag.major,
    npm_dist_tag: parsedTag.channel === "beta" ? "beta" : "latest",
    affected_components: [parsedTag.componentId],
  };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const context = resolveReleaseContext();
  process.stdout.write(`${JSON.stringify(context, null, 2)}\n`);
}
