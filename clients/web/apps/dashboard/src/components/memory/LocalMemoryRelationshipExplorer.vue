<script lang="ts" setup>
import type {
  AdminMemoryEntry,
  LocalMemoryExplorerSelection,
  LocalMemoryRelationshipCluster,
} from "@/types/admin-sessions";

const props = defineProps<{
  clusters: LocalMemoryRelationshipCluster[];
  visibleEntries: AdminMemoryEntry[];
  selection: LocalMemoryExplorerSelection;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const emit = defineEmits<{
  "select-cluster": [cluster: LocalMemoryRelationshipCluster];
  "clear-selection": [];
  "open-browse": [];
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function isClusterSelected(cluster: LocalMemoryRelationshipCluster): boolean {
  return (
      (props.selection.sessionId ?? undefined) === (cluster.sessionId ?? undefined) &&
      props.selection.category === cluster.category
  );
}
</script>

<template>
  <section class="relationship-panel">
    <header class="panel-header">
      <div>
        <p class="eyebrow">Derived local relationship explorer</p>
        <h3>Session ↔ category intersections</h3>
      </div>
      <div class="panel-actions">
        <button v-if="selection.category || selection.sessionId" class="relationship-clear"
                type="button" @click="emit('clear-selection')">
          Clear focus
        </button>
        <button class="relationship-open-browse" type="button" @click="emit('open-browse')">
          Open in browse list
        </button>
      </div>
    </header>

    <p class="helper">This view is inferred from local sessions and categories only, not Cerebro
      semantics.</p>

    <div class="relationship-clusters">
      <button
          v-for="cluster in clusters"
          :key="`${cluster.sessionId ?? 'no-session'}-${cluster.category}`"
          :aria-pressed="isClusterSelected(cluster)"
          :class="{ 'relationship-cluster-active': isClusterSelected(cluster) }"
          class="relationship-cluster"
          type="button"
          @click="emit('select-cluster', cluster)"
      >
        <span>{{ cluster.sessionId ?? "No Session" }}</span>
        <span>{{ cluster.category }}</span>
        <span>{{ cluster.count }}</span>
      </button>
    </div>

    <ul class="relationship-entries">
      <li v-for="entry in visibleEntries" :key="entry.id">
        <strong>{{ entry.key }}</strong>
        <span>{{ entry.category }}</span>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.relationship-panel,
.relationship-clusters,
.relationship-entries {
  display: grid;
  gap: 10px;
}

.panel-header {
  display: flex;
  gap: 12px;
  justify-content: space-between;
}

.panel-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.panel-header h3,
.eyebrow {
  margin: 0;
}

.eyebrow,
.helper {
  color: var(--color-text-secondary);
}

.eyebrow {
  font-size: 11px;
  text-transform: uppercase;
}

.relationship-clear,
.relationship-open-browse,
.relationship-cluster {
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  color: inherit;
}

.relationship-clear,
.relationship-open-browse {
  cursor: pointer;
  padding: 6px 10px;
}

.relationship-cluster {
  cursor: pointer;
  display: grid;
  gap: 8px;
  grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr) auto;
  padding: 10px 12px;
}

.relationship-cluster-active {
  border-color: color-mix(in srgb, var(--color-primary) 45%, var(--color-border));
}

.relationship-entries {
  list-style: none;
  margin: 0;
  padding: 0;
}

.relationship-entries li {
  background: color-mix(in srgb, var(--color-bg-secondary) 80%, transparent);
  border-radius: 8px;
  display: flex;
  gap: 10px;
  justify-content: space-between;
  padding: 8px 10px;
}
</style>
