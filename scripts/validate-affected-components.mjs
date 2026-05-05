import { loadReleaseComponents } from "./release-components.mjs";
import { getKnownComponentIds, sortStrings } from "./release-scope-utils.mjs";

export function validateAffectedComponents({
  affectedComponentsRaw = process.env.AFFECTED_COMPONENTS ?? "[]",
} = {}) {
  let parsed;
  try {
    parsed = JSON.parse(affectedComponentsRaw);
  } catch (error) {
    throw new TypeError(`Invalid AFFECTED_COMPONENTS payload: ${affectedComponentsRaw}`, { cause: error });
  }

  if (!Array.isArray(parsed)) {
    throw new TypeError(`Invalid AFFECTED_COMPONENTS payload: ${affectedComponentsRaw}`);
  }

  const graph = loadReleaseComponents();
  const knownComponents = new Set(getKnownComponentIds(graph));
  const unknownComponents = sortStrings(parsed.filter((componentId) => !knownComponents.has(componentId)));
  if (unknownComponents.length > 0) {
    throw new Error(`Unknown affected components in _publish input: ${unknownComponents.join(", ")}`);
  }

  const affectedComponents = sortStrings([...new Set(parsed)]);
  if (affectedComponents.length === 0) {
    throw new Error("No publishable affected components were provided to _publish");
  }

  return {
    affected_components: affectedComponents,
    has_corvus_runtime: affectedComponents.includes("corvus-runtime"),
    has_rook: affectedComponents.includes("rook"),
    has_cerebro: affectedComponents.includes("cerebro"),
    has_gradle_kmp: affectedComponents.includes("gradle-kmp"),
    raw_payload: affectedComponentsRaw,
  };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const validated = validateAffectedComponents();
  process.stdout.write(`${JSON.stringify(validated, null, 2)}\n`);
}
