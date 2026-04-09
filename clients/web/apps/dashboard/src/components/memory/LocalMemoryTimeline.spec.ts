import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import LocalMemoryTimeline from "@/components/memory/LocalMemoryTimeline.vue";
import type { LocalMemoryTimelineGroup } from "@/types/admin-sessions";

const groups: LocalMemoryTimelineGroup[] = [
  {
    sessionId: null,
    label: "No Session",
    entryCount: 1,
    firstTimestamp: "2026-01-01T10:00:00Z",
    lastTimestamp: "2026-01-01T10:00:00Z",
    categories: { Daily: 1 },
    entries: [
      {
        id: "m1",
        key: "key-1",
        content: "missing session",
        category: "Daily",
        timestamp: "2026-01-01T10:00:00Z",
        session_id: null,
      },
    ],
  },
  {
    sessionId: "session-a",
    label: "session-a",
    entryCount: 2,
    firstTimestamp: "2026-01-02T10:00:00Z",
    lastTimestamp: "2026-01-03T10:00:00Z",
    categories: { Core: 2 },
    entries: [
      {
        id: "m2",
        key: "key-2",
        content: "older",
        category: "Core",
        timestamp: "2026-01-02T10:00:00Z",
        session_id: "session-a",
      },
      {
        id: "m3",
        key: "key-3",
        content: "newer",
        category: "Core",
        timestamp: "2026-01-03T10:00:00Z",
        session_id: "session-a",
      },
    ],
  },
];

describe("LocalMemoryTimeline", () => {
  it("renders session lanes in chronological order with a navigable no-session fallback", () => {
    const wrapper = mount(LocalMemoryTimeline, {
      props: { groups },
    });

    expect(wrapper.findAll("[data-testid='timeline-group']")).toHaveLength(2);
    expect(wrapper.text()).toContain("No Session");
    expect(wrapper.findAll("[data-testid='timeline-entry']").map((entry) => entry.text())).toEqual([
      expect.stringContaining("key-1"),
      expect.stringContaining("key-2"),
      expect.stringContaining("key-3"),
    ]);
  });

  it("emits the selected session when a lane is activated", async () => {
    const wrapper = mount(LocalMemoryTimeline, {
      props: {
        groups,
        activeSessionId: "session-a",
      },
    });

    await wrapper.findAll("button.timeline-group-button")[1]?.trigger("click");

    expect(wrapper.emitted("select-session")).toEqual([["session-a"]]);
    expect(wrapper.find(".timeline-group-active").text()).toContain("session-a");
    expect(wrapper.findAll("button.timeline-group-button")[1]?.attributes("aria-pressed")).toBe(
      "true"
    );
  });
});
