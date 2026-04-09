import { computed, ref } from "vue";

import type { MemoryListParams } from "@/composables/useAdmin";
import type {
  AdminMemoryEntry,
  AdminMemoryListResponse,
  AdminMemoryStats,
  LocalMemoryCategoryFacet,
  LocalMemoryExplorerSelection,
  LocalMemoryExplorerSnapshot,
  LocalMemoryRelationshipCluster,
  LocalMemoryTimelineGroup,
} from "@/types/admin-sessions";

export const MEMORY_EXPLORER_PAGE_SIZE = 200;
export const MEMORY_EXPLORER_MAX_ENTRIES = 600;

const NO_SESSION_LABEL = "No Session";

interface LocalMemoryExplorerApi {
  listMemoryEntries(params?: MemoryListParams): Promise<AdminMemoryListResponse | null>;
  fetchMemoryStats(): Promise<AdminMemoryStats | null>;
}

function normalizeSessionId(sessionId?: string | null): string | undefined {
  if (!sessionId) {
    return undefined;
  }

  const trimmed = sessionId.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

function normalizeSelection(
  selection: LocalMemoryExplorerSelection = {}
): LocalMemoryExplorerSelection {
  return {
    sessionId: normalizeSessionId(selection.sessionId),
    category: selection.category?.trim() || undefined,
    entryId: selection.entryId?.trim() || undefined,
  };
}

function toSessionKey(sessionId?: string | null): string {
  return sessionId ?? "__no_session__";
}

function compareEntries(left: AdminMemoryEntry, right: AdminMemoryEntry): number {
  const leftTimestamp = Number(new Date(left.timestamp));
  const rightTimestamp = Number(new Date(right.timestamp));

  if (leftTimestamp === rightTimestamp) {
    return left.id.localeCompare(right.id);
  }

  return leftTimestamp - rightTimestamp;
}

export function useLocalMemoryExplorer(api: LocalMemoryExplorerApi) {
  const entries = ref<AdminMemoryEntry[]>([]);
  const stats = ref<AdminMemoryStats | null>(null);
  const selection = ref<LocalMemoryExplorerSelection>({});
  const isLoading = ref(false);
  const isTruncated = ref(false);
  const error = ref<string | null>(null);
  const loadedEntries = ref(0);
  const totalEntries = ref(0);

  const visibleEntries = computed(() => {
    const activeSelection = selection.value;

    return [...entries.value].sort(compareEntries).filter((entry) => {
      if (
        activeSelection.sessionId &&
        (entry.session_id ?? undefined) !== activeSelection.sessionId
      ) {
        return false;
      }

      if (activeSelection.category && entry.category !== activeSelection.category) {
        return false;
      }

      return true;
    });
  });

  const timelineGroups = computed<LocalMemoryTimelineGroup[]>(() => {
    const groupSource = [...entries.value].sort(compareEntries).filter((entry) => {
      if (selection.value.category && entry.category !== selection.value.category) {
        return false;
      }

      if (
        selection.value.sessionId &&
        (entry.session_id ?? undefined) !== selection.value.sessionId
      ) {
        return false;
      }

      return true;
    });

    const grouped = new Map<string, AdminMemoryEntry[]>();
    for (const entry of groupSource) {
      const key = toSessionKey(entry.session_id);
      grouped.set(key, [...(grouped.get(key) ?? []), entry]);
    }

    return [...grouped.entries()]
      .map(([_groupKey, groupEntries]) => {
        const firstEntry = groupEntries[0];
        const lastEntry = groupEntries[groupEntries.length - 1];
        const sessionId = groupEntries[0]?.session_id ?? null;
        const categories = groupEntries.reduce<Record<string, number>>((accumulator, entry) => {
          accumulator[entry.category] = (accumulator[entry.category] ?? 0) + 1;
          return accumulator;
        }, {});

        return {
          sessionId,
          label: sessionId ?? NO_SESSION_LABEL,
          entryCount: groupEntries.length,
          firstTimestamp: firstEntry?.timestamp ?? "",
          lastTimestamp: lastEntry?.timestamp ?? "",
          categories,
          entries: groupEntries,
        } satisfies LocalMemoryTimelineGroup;
      })
      .sort(
        (left, right) =>
          Number(new Date(left.firstTimestamp)) - Number(new Date(right.firstTimestamp))
      );
  });

  const categoryFacets = computed<LocalMemoryCategoryFacet[]>(() => {
    const categories = new Set([
      ...Object.keys(stats.value?.by_category ?? {}),
      ...entries.value.map((entry) => entry.category),
    ]);

    return [...categories]
      .map((category) => {
        const matchingEntries = entries.value.filter((entry) => entry.category === category);
        const total = stats.value?.by_category[category] ?? matchingEntries.length;
        const sessionCount = new Set(
          matchingEntries.map((entry) => entry.session_id ?? NO_SESSION_LABEL)
        ).size;

        return {
          category,
          total,
          sessionCount,
          isActive: selection.value.category === category,
        } satisfies LocalMemoryCategoryFacet;
      })
      .sort(
        (left, right) => right.total - left.total || left.category.localeCompare(right.category)
      );
  });

  const relationshipClusters = computed<LocalMemoryRelationshipCluster[]>(() => {
    const clusterSource = [...entries.value].sort(compareEntries).filter((entry) => {
      if (selection.value.category && entry.category !== selection.value.category) {
        return false;
      }

      if (
        selection.value.sessionId &&
        (entry.session_id ?? undefined) !== selection.value.sessionId
      ) {
        return false;
      }

      return true;
    });

    const grouped = new Map<string, AdminMemoryEntry[]>();
    for (const entry of clusterSource) {
      const groupKey = `${toSessionKey(entry.session_id)}::${entry.category}`;
      grouped.set(groupKey, [...(grouped.get(groupKey) ?? []), entry]);
    }

    return [...grouped.values()]
      .map((groupEntries) => ({
        sessionId: groupEntries[0]?.session_id ?? null,
        category: groupEntries[0]?.category ?? "",
        count: groupEntries.length,
        entries: groupEntries,
      }))
      .sort(
        (left, right) =>
          right.count - left.count ||
          compareEntries(left.entries[0] as AdminMemoryEntry, right.entries[0] as AdminMemoryEntry)
      );
  });

  const snapshot = computed<LocalMemoryExplorerSnapshot>(() => ({
    entries: entries.value,
    stats: stats.value,
    timelineGroups: timelineGroups.value,
    categoryFacets: categoryFacets.value,
    relationshipClusters: relationshipClusters.value,
    selection: selection.value,
    loadedEntries: loadedEntries.value,
    totalEntries: totalEntries.value,
    isTruncated: isTruncated.value,
  }));

  async function load(nextSelection: LocalMemoryExplorerSelection = {}) {
    isLoading.value = true;
    error.value = null;
    isTruncated.value = false;
    selection.value = normalizeSelection(nextSelection);

    try {
      stats.value = await api.fetchMemoryStats();

      const collectedEntries: AdminMemoryEntry[] = [];
      let page = 1;
      let currentTotal = 0;

      while (collectedEntries.length < MEMORY_EXPLORER_MAX_ENTRIES) {
        const response = await api.listMemoryEntries({ page, per_page: MEMORY_EXPLORER_PAGE_SIZE });
        if (!response) {
          break;
        }

        currentTotal = response.total;
        collectedEntries.push(...response.entries);

        if (response.entries.length === 0 || collectedEntries.length >= response.total) {
          break;
        }

        page += 1;
      }

      entries.value = collectedEntries.slice(0, MEMORY_EXPLORER_MAX_ENTRIES);
      loadedEntries.value = entries.value.length;
      totalEntries.value = stats.value?.total_entries ?? currentTotal;
      isTruncated.value = totalEntries.value > entries.value.length;
    } catch (loadError: unknown) {
      entries.value = [];
      loadedEntries.value = 0;
      totalEntries.value = 0;
      isTruncated.value = false;
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      isLoading.value = false;
    }
  }

  function setSelection(nextSelection: LocalMemoryExplorerSelection = {}) {
    selection.value = normalizeSelection(nextSelection);
  }

  function selectSession(sessionId?: string | null) {
    selection.value = {
      ...selection.value,
      sessionId: normalizeSessionId(sessionId),
      entryId: undefined,
    };
  }

  function selectCategory(category?: string) {
    selection.value = {
      ...selection.value,
      category: category?.trim() || undefined,
      entryId: undefined,
    };
  }

  function selectCluster(cluster: LocalMemoryRelationshipCluster) {
    selection.value = {
      sessionId: normalizeSessionId(cluster.sessionId),
      category: cluster.category,
      entryId: undefined,
    };
  }

  function clearFocus() {
    selection.value = {};
  }

  return {
    entries,
    stats,
    selection,
    isLoading,
    isTruncated,
    error,
    loadedEntries,
    totalEntries,
    snapshot,
    visibleEntries,
    timelineGroups,
    categoryFacets,
    relationshipClusters,
    load,
    setSelection,
    selectSession,
    selectCategory,
    selectCluster,
    clearFocus,
  };
}
