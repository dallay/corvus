import { loadReleaseComponents } from "./release-components.mjs";
import { getPublishableComponentIds, sortStrings } from "./release-scope-utils.mjs";

function parseAffectedComponentsOverride(releaseBody) {
  const match = /^affected_components\s*:\s*(.+)$/im.exec(releaseBody);
  if (!match) {
    return [];
  }

  return match[1]
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

export function resolveReleaseFromTag({
  releaseTag = process.env.RELEASE_TAG,
  releaseBody = process.env.RELEASE_BODY ?? "",
} = {}) {
  if (!releaseTag) {
    throw new Error("RELEASE_TAG is required");
  }

  const graph = loadReleaseComponents();
  const publishableComponents = new Set(getPublishableComponentIds(graph));

  const overrideComponents = parseAffectedComponentsOverride(releaseBody);
  if (overrideComponents.length > 0) {
    const invalidComponents = overrideComponents.filter((componentId) => !publishableComponents.has(componentId));
    if (invalidComponents.length > 0) {
      throw new Error(`Unsupported affected_components override in release body: ${invalidComponents.join(", ")}`);
    }

    return {
      release_tag: releaseTag,
      affected_components: sortStrings([...new Set(overrideComponents)]),
      supported_release: true,
      resolution_reason: "release body override",
    };
  }

  for (const componentId of sortStrings([...publishableComponents])) {
    if (releaseTag.startsWith(`${componentId}-v`)) {
      return {
        release_tag: releaseTag,
        affected_components: [componentId],
        supported_release: true,
        resolution_reason: "release tag prefix",
      };
    }
  }

  return {
    release_tag: releaseTag,
    affected_components: [],
    supported_release: false,
    resolution_reason: "unsupported release tag",
  };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const resolved = resolveReleaseFromTag();
  process.stdout.write(`${JSON.stringify(resolved, null, 2)}\n`);
}
