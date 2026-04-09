<script setup lang="ts">
import { onMounted, watch } from "vue";

// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import LocalMemoryCategoryChart from "@/components/memory/LocalMemoryCategoryChart.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import LocalMemoryRelationshipExplorer from "@/components/memory/LocalMemoryRelationshipExplorer.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import LocalMemoryTimeline from "@/components/memory/LocalMemoryTimeline.vue";
import { useAdmin } from "@/composables/useAdmin";
import { useLocalMemoryExplorer } from "@/composables/useLocalMemoryExplorer";
import type { LocalMemoryExplorerSelection } from "@/types/admin-sessions";

const props = defineProps<{
  gatewayUrl: (path: string) => string;
  authHeaders: () => Record<string, string>;
  selection?: LocalMemoryExplorerSelection;
}>();

const emit = defineEmits<{
  "selection-change": [selection: LocalMemoryExplorerSelection];
  "open-browse": [selection: LocalMemoryExplorerSelection];
}>();

const admin = useAdmin(props.gatewayUrl, props.authHeaders);
const explorer = useLocalMemoryExplorer({
  listMemoryEntries: admin.listMemoryEntries,
  fetchMemoryStats: admin.fetchMemoryStats,
});

watch(
  () => props.selection,
  (nextSelection) => {
    explorer.setSelection(nextSelection ?? {});
  },
  { deep: true }
);

watch(
  () => explorer.selection.value,
  (nextSelection) => {
    emit("selection-change", { ...nextSelection });
  },
  { deep: true }
);

onMounted(async () => {
  await explorer.load(props.selection ?? {});
});
</script>

<template>
  <section class="explorer-panel">
    <header class="explorer-header">
      <div>
        <p class="eyebrow">Local Memory Visualization</p>
        <h3>Explore timeline, categories, and inferred local relationships</h3>
      </div>
      <p class="helper">This surface is derived from local sessions and categories only — not a Cerebro relationship graph.</p>
    </header>

    <p v-if="explorer.isLoading.value" class="helper" role="status">Loading local memory visualization…</p>
    <p v-else-if="explorer.error.value" class="error" role="alert">{{ explorer.error.value }}</p>
    <div v-else-if="explorer.snapshot.value.totalEntries === 0" class="empty-state">
      <p>No local memory entries are available to visualize yet.</p>
      <p class="helper">The local visualization remains derived from local sessions and categories only.</p>
    </div>
    <template v-else>
      <p v-if="explorer.isTruncated.value" class="helper truncation-notice">
        Showing {{ explorer.snapshot.value.loadedEntries }} of {{ explorer.snapshot.value.totalEntries }} local entries.
      </p>

      <div class="explorer-grid">
        <LocalMemoryCategoryChart
          :facets="explorer.categoryFacets.value"
          @select-category="explorer.selectCategory"
          @clear-category="explorer.clearFocus"
        />
        <LocalMemoryTimeline
          :groups="explorer.timelineGroups.value"
          :active-session-id="explorer.selection.value.sessionId"
          :active-category="explorer.selection.value.category"
          @select-session="explorer.selectSession"
        />
      </div>

      <LocalMemoryRelationshipExplorer
        :clusters="explorer.relationshipClusters.value"
        :visible-entries="explorer.visibleEntries.value"
        :selection="explorer.selection.value"
        @select-cluster="explorer.selectCluster"
        @clear-selection="explorer.clearFocus"
        @open-browse="emit('open-browse', { ...explorer.selection.value })"
      />
    </template>
  </section>
</template>

<style scoped>
.explorer-panel,
.explorer-grid,
.empty-state {
  display: grid;
  gap: 14px;
}

.explorer-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 12px;
}

.explorer-header h3,
.eyebrow,
.empty-state p {
  margin: 0;
}

.eyebrow {
  font-size: 11px;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.helper {
  color: var(--color-text-secondary);
}

.explorer-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.truncation-notice {
  margin: 0;
}

@media (max-width: 900px) {
  .explorer-grid,
  .explorer-header {
    grid-template-columns: 1fr;
    display: grid;
  }
}
</style>
