import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import CerebroSearchPanel from "@/components/memory/CerebroSearchPanel.vue";
import { i18nConfig } from "@/i18n";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

function mountPanel(state: "available" | "unconfigured" | "unreachable" = "available") {
  return mount(CerebroSearchPanel, {
    props: {
      gatewayUrl: (path: string) => new URL(`http://localhost:3000/api${path}`).toString(),
      authHeaders: () => ({ Authorization: "Bearer token" }),
      status: {
        service_state: state,
        tools: {
          mem_search: { state, message: state === "available" ? "ready" : `search is ${state}` },
          mem_get_observation: { state: "available" },
          mem_timeline: { state: "available" },
          mem_stats: { state: "available" },
          mem_save: { state: "available" },
          mem_update: { state: "available" },
          mem_delete: { state: "available" },
          mem_save_prompt: { state: "not_implemented" },
          mem_session_start: { state: "not_implemented" },
          mem_session_end: { state: "not_implemented" },
          mem_session_summary: { state: "not_implemented" },
          mem_context: { state: "not_implemented" },
        },
      },
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("CerebroSearchPanel", () => {
  it("submits semantic search when available", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          state: "available",
          results: [{ memory_id: "mem-42", summary: "dark mode", score: 0.9 }],
          truncated: false,
          results_count: 1,
        }),
        { status: 200, headers: { "Content-Type": "application/json" } }
      )
    );

    const wrapper = mountPanel();
    await wrapper.find("input").setValue("dark mode");
    await wrapper.find("button").trigger("click");
    await flushPromises();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(url).toContain("/web/admin/cerebro/search");
    expect(init?.method).toBe("POST");
    expect(JSON.parse(String(init?.body))).toMatchObject({ query: "dark mode", limit: 8 });
    expect(wrapper.text()).toContain("dark mode");
  });

  it("shows explicit degraded state when unavailable", async () => {
    const wrapper = mountPanel("unconfigured");
    await flushPromises();

    expect(wrapper.find("button").attributes("disabled")).toBeDefined();
    expect(wrapper.text()).toContain("search is unconfigured");
  });
});
