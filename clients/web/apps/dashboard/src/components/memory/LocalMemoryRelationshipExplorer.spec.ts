import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import LocalMemoryRelationshipExplorer from "@/components/memory/LocalMemoryRelationshipExplorer.vue";

const clusters = [
  {
    sessionId: "session-a",
    category: "Core",
    count: 2,
    entries: [
      {
        id: "m1",
        key: "key-1",
        content: "first",
        category: "Core",
        timestamp: "2026-01-01T00:00:00Z",
        session_id: "session-a",
      },
      {
        id: "m2",
        key: "key-2",
        content: "second",
        category: "Core",
        timestamp: "2026-01-02T00:00:00Z",
        session_id: "session-a",
      },
    ],
  },
];

describe("LocalMemoryRelationshipExplorer", () => {
  it("renders inferred relationship clusters and visible entries", async () => {
    const wrapper = mount(LocalMemoryRelationshipExplorer, {
      props: {
        clusters,
        visibleEntries: clusters[0]?.entries ?? [],
        selection: {},
      },
    });

    await wrapper.find("button.relationship-cluster").trigger("click");

    expect(wrapper.text()).toContain("Derived local relationship explorer");
    expect(wrapper.text()).toContain("session-a");
    expect(wrapper.text()).toContain("key-1");
    expect(wrapper.emitted("select-cluster")).toEqual([[clusters[0]]]);
  });

  it("offers clear-focus and browse handoff actions", async () => {
    const wrapper = mount(LocalMemoryRelationshipExplorer, {
      props: {
        clusters,
        visibleEntries: clusters[0]?.entries ?? [],
        selection: { category: "Core", sessionId: "session-a" },
      },
    });

    await wrapper.find("button.relationship-clear").trigger("click");
    await wrapper.find("button.relationship-open-browse").trigger("click");

    expect(wrapper.emitted("clear-selection")).toEqual([[]]);
    expect(wrapper.emitted("open-browse")).toEqual([[]]);
  });
});
