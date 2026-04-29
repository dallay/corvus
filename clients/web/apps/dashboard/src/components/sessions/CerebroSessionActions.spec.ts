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
          mem_suggest_topic_key: { state: "available" },
          mem_save_prompt: { state: "not_implemented", message: "Prompt save is planned." },
          mem_session_start: { state: "not_implemented", message: "Session start is planned." },
          mem_session_end: { state: "not_implemented", message: "Session end is planned." },
          mem_session_summary: {
            state: "not_implemented",
            message: "Summary generation is planned.",
          },
          mem_context: { state: "not_implemented", message: "Context lookup is deferred." },
        },
      },
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("CerebroSessionActions", () => {
  it("shows deferred session tools explicitly without blocking local session UI", async () => {
    const wrapper = mountActions();
    await flushPromises();

    expect(wrapper.text()).toContain("Session Summary");
    expect(wrapper.text()).toContain("not_implemented");
    expect(wrapper.text()).toContain("Context Lookup");
    expect(wrapper.findAll("button").some((button) => button.text() === "Run")).toBe(false);

    const buttons = wrapper.findAll("button");
    expect(buttons).toHaveLength(4);
    expect(buttons.map((button) => button.text())).toEqual([
      "not_implemented",
      "not_implemented",
      "not_implemented",
      "not_implemented",
    ]);
  });

  it("disables deferred context lookup and surfaces deferred messaging", async () => {
    const wrapper = mountActions();
    await flushPromises();

    const contextRow = wrapper.findAll("li").find((row) => row.text().includes("Context Lookup"));

    expect(contextRow?.text()).toContain("Context lookup is deferred.");
    expect(contextRow?.find("button").attributes("disabled")).toBeDefined();
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
