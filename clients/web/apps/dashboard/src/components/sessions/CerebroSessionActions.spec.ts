import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import CerebroSessionActions from "@/components/sessions/CerebroSessionActions.vue";
import { i18nConfig } from "@/i18n";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

function mountActions() {
  return mount(CerebroSessionActions, {
    props: {
      gatewayUrl: (path: string) => new URL(`http://localhost:3000/api${path}`).toString(),
      authHeaders: () => ({ Authorization: "Bearer token" }),
      sessionId: "abc-123",
      status: {
        service_state: "available",
        tools: {
          mem_search: { state: "available" },
          mem_get_observation: { state: "available" },
          mem_timeline: { state: "available" },
          mem_stats: { state: "available" },
          mem_save: { state: "available" },
          mem_update: { state: "available" },
          mem_delete: { state: "available" },
          mem_save_prompt: { state: "not_implemented", message: "Prompt save is planned." },
          mem_session_start: { state: "not_implemented", message: "Session start is planned." },
          mem_session_end: { state: "not_implemented", message: "Session end is planned." },
          mem_session_summary: {
            state: "not_implemented",
            message: "Summary generation is planned.",
          },
          mem_context: { state: "available", message: "Context is ready." },
        },
      },
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("CerebroSessionActions", () => {
  it("shows planned tools explicitly without blocking local session UI", async () => {
    const wrapper = mountActions();
    await flushPromises();

    expect(wrapper.text()).toContain("Session Summary");
    expect(wrapper.text()).toContain("not_implemented");
    expect(wrapper.text()).toContain("Context Lookup");
  });

  it("invokes available context lookup through the typed endpoint", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          state: "available",
          tool: "mem_context",
          data: { items: [{ summary: "dark mode" }] },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } }
      )
    );

    const wrapper = mountActions();
    await flushPromises();

    await wrapper
      .findAll("button")
      .find((button) => button.text() === "Run")
      ?.trigger("click");
    await flushPromises();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(url).toContain("/web/admin/cerebro/context");
    expect(init?.method).toBe("POST");
    expect(wrapper.text()).toContain("dark mode");
  });
});
