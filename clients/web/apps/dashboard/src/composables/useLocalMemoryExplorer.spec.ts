import { describe, expect, it, vi } from "vitest";

import {
  MEMORY_EXPLORER_MAX_ENTRIES,
  useLocalMemoryExplorer,
} from "@/composables/useLocalMemoryExplorer";
import type {
  AdminMemoryEntry,
  AdminMemoryListResponse,
  AdminMemoryStats,
} from "@/types/admin-sessions";

function createEntry(id: string, overrides: Partial<AdminMemoryEntry> = {}): AdminMemoryEntry {
  return {
    id,
    key: `key-${id}`,
    content: `content-${id}`,
    category: "Core",
    timestamp: `2026-01-${String(Number(id.replace(/\D/g, "") || 1)).padStart(2, "0")}T00:00:00Z`,
    session_id: "session-a",
    ...overrides,
  };
}

function createListResponse(
  entries: AdminMemoryEntry[],
  total = entries.length,
  offset = 0
): AdminMemoryListResponse {
  return {
    entries,
    total,
    limit: entries.length,
    offset,
  };
}

const stats: AdminMemoryStats = {
  total_entries: 4,
  by_category: { Core: 2, Conversation: 1, Daily: 1 },
  total_sessions: 2,
  active_sessions: 1,
  backend: "sqlite",
  cerebro_configured: false,
};

describe("useLocalMemoryExplorer", () => {
  it("builds chronological timeline groups including the no-session fallback lane", async () => {
    const listMemoryEntries = vi
      .fn<(...args: unknown[]) => Promise<AdminMemoryListResponse | null>>()
      .mockResolvedValue(
        createListResponse([
          createEntry("2", {
            timestamp: "2026-01-02T09:00:00Z",
            session_id: "session-b",
            category: "Conversation",
          }),
          createEntry("1", {
            timestamp: "2026-01-01T09:00:00Z",
            session_id: null,
            category: "Daily",
          }),
          createEntry("3", {
            timestamp: "2026-01-03T09:00:00Z",
            session_id: "session-b",
            category: "Core",
          }),
          createEntry("4", {
            timestamp: "2026-01-04T09:00:00Z",
            session_id: "session-a",
            category: "Core",
          }),
        ])
      );
    const fetchMemoryStats = vi
      .fn<() => Promise<AdminMemoryStats | null>>()
      .mockResolvedValue(stats);

    const explorer = useLocalMemoryExplorer({ listMemoryEntries, fetchMemoryStats });
    await explorer.load();

    expect(explorer.timelineGroups.value.map((group) => group.label)).toEqual([
      "No Session",
      "session-b",
      "session-a",
    ]);
    expect(explorer.timelineGroups.value[1]?.entries.map((entry) => entry.id)).toEqual(["2", "3"]);
  });

  it("supports category focus, clear focus, and session-category intersections without Cerebro data", async () => {
    const listMemoryEntries = vi
      .fn<(...args: unknown[]) => Promise<AdminMemoryListResponse | null>>()
      .mockResolvedValue(
        createListResponse([
          createEntry("1", { session_id: "session-a", category: "Core" }),
          createEntry("2", { session_id: "session-a", category: "Conversation" }),
          createEntry("3", { session_id: "session-b", category: "Core" }),
        ])
      );
    const fetchMemoryStats = vi.fn<() => Promise<AdminMemoryStats | null>>().mockResolvedValue({
      ...stats,
      total_entries: 3,
      by_category: { Core: 2, Conversation: 1 },
    });

    const explorer = useLocalMemoryExplorer({ listMemoryEntries, fetchMemoryStats });
    await explorer.load();

    explorer.selectCategory("Core");
    expect(explorer.selection.value.category).toBe("Core");
    expect(explorer.visibleEntries.value.map((entry) => entry.id)).toEqual(["1", "3"]);

    explorer.selectSession("session-a");
    expect(explorer.relationshipClusters.value).toEqual([
      expect.objectContaining({ sessionId: "session-a", category: "Core", count: 1 }),
    ]);

    explorer.clearFocus();
    expect(explorer.selection.value).toEqual({});
    expect(explorer.visibleEntries.value.map((entry) => entry.id)).toEqual(["1", "2", "3"]);
  });

  it("marks the explorer as truncated when the dashboard-side cap is reached", async () => {
    const firstBatch = Array.from({ length: 200 }, (_, index) =>
      createEntry(String(index + 1), { session_id: `session-${index % 3}` })
    );
    const secondBatch = Array.from({ length: 200 }, (_, index) =>
      createEntry(String(index + 201), { session_id: `session-${index % 3}` })
    );
    const thirdBatch = Array.from({ length: 200 }, (_, index) =>
      createEntry(String(index + 401), { session_id: `session-${index % 3}` })
    );
    const fourthBatch = Array.from({ length: 200 }, (_, index) =>
      createEntry(String(index + 601), { session_id: `session-${index % 3}` })
    );

    const listMemoryEntries = vi
      .fn<(...args: unknown[]) => Promise<AdminMemoryListResponse | null>>()
      .mockResolvedValueOnce(createListResponse(firstBatch, 800, 0))
      .mockResolvedValueOnce(createListResponse(secondBatch, 800, 200))
      .mockResolvedValueOnce(createListResponse(thirdBatch, 800, 400))
      .mockResolvedValueOnce(createListResponse(fourthBatch, 800, 600));
    const fetchMemoryStats = vi.fn<() => Promise<AdminMemoryStats | null>>().mockResolvedValue({
      ...stats,
      total_entries: 800,
    });

    const explorer = useLocalMemoryExplorer({ listMemoryEntries, fetchMemoryStats });
    await explorer.load();

    expect(explorer.isTruncated.value).toBe(true);
    expect(explorer.entries.value).toHaveLength(MEMORY_EXPLORER_MAX_ENTRIES);
    expect(listMemoryEntries).toHaveBeenCalledTimes(3);
  });
});
