<script setup lang="ts">
import type {
  AdminMemoryEntry,
  LocalMemoryExplorerSelection,
  LocalMemoryRelationshipCluster,
} from "@/types/admin-sessions";

defineProps<{
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
</script>

<template>
  <section class="relationship-panel">
    <header class="panel-header">
      <div>
        <p class="eyebrow">Derived local relationship explorer</p>
        <h3>Session ↔ category intersections</h3>
      </div>
      <div class="panel-actions">
        <button v-if="selection.category || selection.sessionId" type="button" class="relationship-clear" @click="emit('clear-selection')">
          Clear focus
        </button>
        <button type="button" class="relationship-open-browse" @click="emit('open-browse')">
          Open in browse list
        </button>
      </div>
    </header>

    <p class="helper">This view is inferred from local sessions and categories only, not Cerebro semantics.</p>

    <div class="relationship-clusters">
      <button
        v-for="cluster in clusters"
        :key="`${cluster.sessionId ?? 'no-session'}-${cluster.category}`"
        type="button"
        class="relationship-cluster"
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
  justify-content: space-between;
  gap: 12px;
}

.panel-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
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
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: transparent;
  color: inherit;
}

.relationship-clear,
.relationship-open-browse {
  padding: 6px 10px;
  cursor: pointer;
}

.relationship-cluster {
  display: grid;
  grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr) auto;
  gap: 8px;
  padding: 10px 12px;
  cursor: pointer;
}

.relationship-entries {
  list-style: none;
  margin: 0;
  padding: 0;
}

.relationship-entries li {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--color-bg-secondary) 80%, transparent);
}
</style>
