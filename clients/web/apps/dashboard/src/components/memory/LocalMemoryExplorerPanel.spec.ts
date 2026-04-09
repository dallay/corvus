import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { computed, ref } from "vue";

import LocalMemoryExplorerPanel from "@/components/memory/LocalMemoryExplorerPanel.vue";
import type {
  AdminMemoryEntry,
  AdminMemoryStats,
  LocalMemoryCategoryFacet,
  LocalMemoryExplorerSelection,
  LocalMemoryRelationshipCluster,
  LocalMemoryTimelineGroup,
} from "@/types/admin-sessions";

const useAdminMock = vi.fn();
const explorerState = vi.hoisted(() => ({
  current: null as ReturnType<typeof createExplorerMock> | null,
}));

vi.mock("@/composables/useAdmin", () => ({
  useAdmin: (...args: unknown[]) => useAdminMock(...args),
}));

vi.mock("@/composables/useLocalMemoryExplorer", () => ({
  useLocalMemoryExplorer: () => {
    if (!explorerState.current) {
      throw new Error("explorer mock not configured");
    }

    return explorerState.current;
  },
}));

function createExplorerMock(overrides: Partial<Record<string, unknown>> = {}) {
  const selection = ref({
    category: undefined as string | undefined,
    sessionId: undefined as string | undefined,
  });
  const snapshot = computed(() => ({
    entries: [],
    stats: null,
    timelineGroups: [],
    categoryFacets: [],
    relationshipClusters: [],
    selection: selection.value,
    loadedEntries: 0,
    totalEntries: 0,
    isTruncated: false,
    ...((overrides.snapshot as Record<string, unknown> | undefined) ?? {}),
  }));

  return {
    isLoading: ref(Boolean(overrides.isLoading)),
    isTruncated: ref(Boolean(overrides.isTruncated)),
    error: ref((overrides.error as string | null | undefined) ?? null),
    selection,
    snapshot,
    visibleEntries: ref(
      (overrides.visibleEntries as Array<Record<string, unknown>> | undefined) ?? []
    ),
    timelineGroups: ref((snapshot.value.timelineGroups as Array<Record<string, unknown>>) ?? []),
    categoryFacets: ref((snapshot.value.categoryFacets as Array<Record<string, unknown>>) ?? []),
    relationshipClusters: ref(
      (snapshot.value.relationshipClusters as Array<Record<string, unknown>>) ?? []
    ),
    load: vi.fn(async (nextSelection?: Record<string, unknown>) => {
      selection.value = {
        ...selection.value,
        ...(nextSelection ?? {}),
      };
    }),
    setSelection: vi.fn((nextSelection?: Record<string, unknown>) => {
      selection.value = {
        ...selection.value,
        ...(nextSelection ?? {}),
      };
    }),
    selectSession: vi.fn(),
    selectCategory: vi.fn(),
    selectCluster: vi.fn(),
    clearFocus: vi.fn(),
  };
}

function createInteractiveExplorerMock() {
  const entries: AdminMemoryEntry[] = [
    {
      id: "m1",
      key: "key-1",
      content: "Core memory in session A",
      category: "Core",
      timestamp: "2026-01-01T00:00:00Z",
      session_id: "session-a",
    },
    {
      id: "m2",
      key: "key-2",
      content: "Conversation memory in session A",
      category: "Conversation",
      timestamp: "2026-01-02T00:00:00Z",
      session_id: "session-a",
    },
    {
      id: "m3",
      key: "key-3",
      content: "Core memory in session B",
      category: "Core",
      timestamp: "2026-01-03T00:00:00Z",
      session_id: "session-b",
    },
  ];
  const stats: AdminMemoryStats = {
    total_entries: 3,
    by_category: {
      Core: 2,
      Conversation: 1,
    },
    total_sessions: 2,
    active_sessions: 1,
    backend: "sqlite",
    cerebro_configured: false,
  };
  const selection = ref<LocalMemoryExplorerSelection>({});
  const visibleEntries = ref<AdminMemoryEntry[]>(entries);
  const timelineGroups = ref<LocalMemoryTimelineGroup[]>([]);
  const categoryFacets = ref<LocalMemoryCategoryFacet[]>([]);
  const relationshipClusters = ref<LocalMemoryRelationshipCluster[]>([]);

  const rebuild = () => {
    const activeCategory = selection.value.category;
    const activeSessionId = selection.value.sessionId;

    visibleEntries.value = entries.filter((entry) => {
      const matchesCategory = !activeCategory || entry.category === activeCategory;
      const matchesSession = !activeSessionId || entry.session_id === activeSessionId;

      return matchesCategory && matchesSession;
    });

    timelineGroups.value = ["session-a", "session-b"].map((sessionId) => {
      const sessionEntries = entries.filter((entry) => entry.session_id === sessionId);

      return {
        sessionId,
        label: sessionId,
        entryCount: sessionEntries.length,
        firstTimestamp: sessionEntries[0]?.timestamp ?? "",
        lastTimestamp: sessionEntries.at(-1)?.timestamp ?? "",
        categories: sessionEntries.reduce<Record<string, number>>((acc, entry) => {
          acc[entry.category] = (acc[entry.category] ?? 0) + 1;

          return acc;
        }, {}),
        entries: sessionEntries,
      };
    });

    categoryFacets.value = Object.entries(stats.by_category).map(([category, total]) => ({
      category,
      total,
      sessionCount: new Set(
        entries
          .filter((entry) => entry.category === category)
          .map((entry) => entry.session_id ?? "")
      ).size,
      isActive: selection.value.category === category,
    }));

    relationshipClusters.value = [
      {
        sessionId: "session-a",
        category: "Core",
        count: 1,
        entries: entries.filter(
          (entry) => entry.session_id === "session-a" && entry.category === "Core"
        ),
      },
      {
        sessionId: "session-a",
        category: "Conversation",
        count: 1,
        entries: entries.filter(
          (entry) => entry.session_id === "session-a" && entry.category === "Conversation"
        ),
      },
      {
        sessionId: "session-b",
        category: "Core",
        count: 1,
        entries: entries.filter(
          (entry) => entry.session_id === "session-b" && entry.category === "Core"
        ),
      },
    ].filter((cluster) => {
      const matchesCategory = !activeCategory || cluster.category === activeCategory;
      const matchesSession = !activeSessionId || cluster.sessionId === activeSessionId;

      return matchesCategory && matchesSession;
    });
  };

  rebuild();

  const applySelection = (nextSelection?: LocalMemoryExplorerSelection) => {
    selection.value = {
      ...selection.value,
      ...nextSelection,
    };

    if (!nextSelection?.category) {
      delete selection.value.category;
    }

    if (!nextSelection?.sessionId) {
      delete selection.value.sessionId;
    }

    if (!nextSelection?.entryId) {
      delete selection.value.entryId;
    }

    rebuild();
  };

  return {
    isLoading: ref(false),
    isTruncated: ref(false),
    error: ref<string | null>(null),
    selection,
    snapshot: computed(() => ({
      entries,
      stats,
      timelineGroups: timelineGroups.value,
      categoryFacets: categoryFacets.value,
      relationshipClusters: relationshipClusters.value,
      selection: selection.value,
      loadedEntries: entries.length,
      totalEntries: entries.length,
      isTruncated: false,
    })),
    visibleEntries,
    timelineGroups,
    categoryFacets,
    relationshipClusters,
    load: vi.fn(async (nextSelection?: LocalMemoryExplorerSelection) => {
      applySelection(nextSelection);
    }),
    setSelection: vi.fn((nextSelection?: LocalMemoryExplorerSelection) => {
      applySelection(nextSelection);
    }),
    selectSession: vi.fn((sessionId?: string) => {
      applySelection({
        ...selection.value,
        sessionId,
      });
    }),
    selectCategory: vi.fn((category: string) => {
      applySelection({
        ...selection.value,
        category,
      });
    }),
    selectCluster: vi.fn((cluster: LocalMemoryRelationshipCluster) => {
      applySelection({
        ...selection.value,
        sessionId: cluster.sessionId ?? undefined,
        category: cluster.category,
        entryId: cluster.entries[0]?.id,
      });
    }),
    clearFocus: vi.fn(() => {
      selection.value = {};
      rebuild();
    }),
  };
}

function mountPanel(props: Record<string, unknown> = {}) {
  useAdminMock.mockReturnValue({
    listMemoryEntries: vi.fn(),
    fetchMemoryStats: vi.fn(),
  });

  return mount(LocalMemoryExplorerPanel, {
    props: {
      gatewayUrl: (path: string) => new URL(`http://localhost:3000/api${path}`).toString(),
      authHeaders: () => ({ Authorization: "Bearer token" }),
      selection: {},
      ...props,
    },
  });
}

describe("LocalMemoryExplorerPanel", () => {
  beforeEach(() => {
    useAdminMock.mockReset();
    explorerState.current = createExplorerMock();
  });

  it("renders a loading state while the local visualization snapshot is loading", async () => {
    explorerState.current = createExplorerMock({ isLoading: true });

    const wrapper = mountPanel();
    await flushPromises();

    expect(wrapper.text()).toContain("Loading local memory visualization");
  });

  it("renders an empty local-only state when no memory entries are available", async () => {
    explorerState.current = createExplorerMock({
      snapshot: {
        entries: [],
        totalEntries: 0,
        loadedEntries: 0,
      },
    });

    const wrapper = mountPanel();
    await flushPromises();

    expect(wrapper.text()).toContain("Local Memory Visualization");
    expect(wrapper.text()).toContain("No local memory entries are available to visualize yet");
    expect(wrapper.text()).toContain("derived from local sessions and categories only");
  });

  it("renders an error state from the explorer composable", async () => {
    explorerState.current = createExplorerMock({ error: "HTTP 500" });

    const wrapper = mountPanel();
    await flushPromises();

    expect(wrapper.text()).toContain("HTTP 500");
  });

  it("surfaces a truncation notice when the bounded local snapshot omits additional entries", async () => {
    explorerState.current = createExplorerMock({
      isTruncated: true,
      snapshot: {
        entries: [{ id: "m1" }],
        totalEntries: 800,
        loadedEntries: 600,
        isTruncated: true,
      },
      visibleEntries: [
        {
          id: "m1",
          key: "key-m1",
          content: "c",
          category: "Core",
          timestamp: "2026-01-01T00:00:00Z",
          session_id: "s1",
        },
      ],
    });

    const wrapper = mountPanel();
    await flushPromises();

    expect(wrapper.text()).toContain("Showing 600 of 800 local entries");
  });

  it("coordinates category focus, relationship drill-in, and browse handoff across the real child panels", async () => {
    explorerState.current = createInteractiveExplorerMock();

    const wrapper = mountPanel();
    await flushPromises();

    expect(wrapper.text()).toContain("key-2");

    const categoryButtons = wrapper.findAll("button.category-bar");
    await categoryButtons[0]?.trigger("click");
    await flushPromises();

    expect(wrapper.find("button.clear-category-focus").exists()).toBe(true);
    expect(wrapper.text()).toContain("key-1");
    expect(wrapper.text()).toContain("key-3");
    expect(wrapper.text()).not.toContain("key-2");

    const groupTexts = wrapper.findAll("[data-testid='timeline-group']").map((node) => node.text());
    expect(groupTexts[0]).toContain("key-1");
    expect(groupTexts[0]).not.toContain("key-2");

    const clusterButtons = wrapper.findAll("button.relationship-cluster");
    await clusterButtons[0]?.trigger("click");
    await flushPromises();

    const lastSelectionChange = wrapper.emitted("selection-change")?.at(-1);
    expect(lastSelectionChange).toEqual([
      {
        category: "Core",
        sessionId: "session-a",
        entryId: "m1",
      },
    ]);
    expect(wrapper.find(".timeline-group-active").text()).toContain("session-a");
    expect(wrapper.find(".relationship-entries").text()).toContain("key-1");
    expect(wrapper.find(".relationship-entries").text()).not.toContain("key-3");

    await wrapper.find("button.relationship-open-browse").trigger("click");

    expect(wrapper.emitted("open-browse")?.at(-1)).toEqual([
      {
        category: "Core",
        sessionId: "session-a",
        entryId: "m1",
      },
    ]);
  });
});
