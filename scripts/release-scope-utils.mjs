const stableStringCollator = new Intl.Collator("en", {
  numeric: true,
  sensitivity: "base",
});

export function sortStrings(values) {
  return [...values].sort((left, right) => stableStringCollator.compare(left, right));
}

export function getPublishableComponentIds(graph) {
  return Object.entries(graph.components)
    .filter(([, component]) => component.publishPolicy === "publishable")
    .map(([componentId]) => componentId);
}

export function getKnownComponentIds(graph) {
  return Object.keys(graph.components);
}
