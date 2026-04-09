import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

import CerebroTimelinePanel from "@/components/memory/CerebroTimelinePanel.vue";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

describe("CerebroTimelinePanel", () => {
  it("renders timeline items from the typed endpoint", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          state: "available",
          items: [{ id: "event-1", timestamp: "2026-04-09T00:00:00Z", summary: "Saved" }],
          items_count: 1,
        }),
        { status: 200, headers: { "Content-Type": "application/json" } }
      )
    );

    const wrapper = mount(CerebroTimelinePanel, {
      props: {
        gatewayUrl: (path: string) => new URL(`http://localhost:3000/api${path}`).toString(),
        authHeaders: () => ({ Authorization: "Bearer token" }),
        selected: { memory_id: "mem-42", summary: "dark mode" },
      },
    });

    await flushPromises();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain("event-1");
    expect(wrapper.text()).toContain("Saved");
  });
});
