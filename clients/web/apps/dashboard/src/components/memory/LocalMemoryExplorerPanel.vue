<script lang="ts" setup>
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

function normalizeSelection(
  selection: LocalMemoryExplorerSelection = {}
): LocalMemoryExplorerSelection {
  return {
    sessionId: selection.sessionId?.trim() || undefined,
    category: selection.category?.trim() || undefined,
    entryId: selection.entryId?.trim() || undefined,
  };
}

function selectionsEqual(
  left: LocalMemoryExplorerSelection = {},
  right: LocalMemoryExplorerSelection = {}
): boolean {
  return JSON.stringify(normalizeSelection(left)) === JSON.stringify(normalizeSelection(right));
}

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
  (nextSelection, previousSelection) => {
    const normalizedNextSelection = normalizeSelection(nextSelection ?? {});

    if (
      selectionsEqual(nextSelection ?? {}, previousSelection ?? {}) ||
      selectionsEqual(normalizedNextSelection, explorer.selection.value)
    ) {
      return;
    }

    explorer.setSelection(normalizedNextSelection);
  }
);

watch(
  () => explorer.selection.value,
  (nextSelection, previousSelection) => {
    if (selectionsEqual(nextSelection, previousSelection ?? {})) {
      return;
    }

    emit("selection-change", { ...nextSelection });
  }
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
      <p class="helper">This surface is derived from local sessions and categories only — not a
        Cerebro relationship graph.</p>
    </header>

    <p v-if="explorer.isLoading.value" aria-live="polite" class="helper">Loading local memory
      visualization…</p>
    <p v-else-if="explorer.error.value" aria-live="assertive" class="error">{{ explorer.error.value }}</p>
    <div v-else-if="explorer.snapshot.value.totalEntries === 0" class="empty-state">
      <p>No local memory entries are available to visualize yet.</p>
      <p class="helper">The local visualization remains derived from local sessions and categories
        only.</p>
    </div>
    <template v-else>
      <p v-if="explorer.isTruncated.value" class="helper truncation-notice">
        Showing {{ explorer.snapshot.value.loadedEntries }} of
        {{ explorer.snapshot.value.totalEntries }} local entries.
      </p>

      <div class="explorer-grid">
        <LocalMemoryCategoryChart
            :facets="explorer.categoryFacets.value"
            @select-category="explorer.selectCategory"
            @clear-category="explorer.clearFocus"
        />
        <LocalMemoryTimeline
            :active-category="explorer.selection.value.category"
            :active-session-id="explorer.selection.value.sessionId"
            :groups="explorer.timelineGroups.value"
            @select-session="explorer.selectSession"
        />
      </div>

      <LocalMemoryRelationshipExplorer
          :clusters="explorer.relationshipClusters.value"
          :selection="explorer.selection.value"
          :visible-entries="explorer.visibleEntries.value"
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
  align-items: flex-start;
  display: flex;
  gap: 12px;
  justify-content: space-between;
}

.explorer-header h3,
.eyebrow,
.empty-state p {
  margin: 0;
}

.eyebrow {
  color: var(--color-text-secondary);
  font-size: 11px;
  text-transform: uppercase;
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
    display: grid;
    grid-template-columns: 1fr;
  }
}
</style>
