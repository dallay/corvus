import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import MemoryStats from "@/components/memory/MemoryStats.vue";
import { i18nConfig } from "@/i18n";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

function queueDefaultRequests(options?: { remoteUnavailable?: boolean }) {
  fetchMock
    .mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          total_entries: 50,
          by_category: { Core: 20, Conversation: 15 },
          total_sessions: 8,
          active_sessions: 3,
          backend: "sqlite",
          cerebro_configured: true,
        }),
        { status: 200, headers: { "Content-Type": "application/json" } }
      )
    )
    .mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          service_state: options?.remoteUnavailable ? "unreachable" : "available",
          tools: {
            mem_search: { state: options?.remoteUnavailable ? "unreachable" : "available" },
            mem_get_observation: { state: options?.remoteUnavailable ? "unreachable" : "available" },
            mem_timeline: { state: options?.remoteUnavailable ? "unreachable" : "available" },
            mem_stats: { state: options?.remoteUnavailable ? "unreachable" : "available" },
            mem_save: { state: "available" },
            mem_update: { state: "available" },
            mem_delete: { state: "available" },
            mem_save_prompt: { state: "not_implemented" },
            mem_session_start: { state: "not_implemented" },
            mem_session_end: { state: "not_implemented" },
            mem_session_summary: { state: "not_implemented" },
            mem_context: { state: "not_implemented" },
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } }
      )
    )
    .mockResolvedValueOnce(
      new Response(
        JSON.stringify(
          options?.remoteUnavailable
            ? {
                state: "unreachable",
                tool: "mem_stats",
                message: "Cerebro is currently unreachable.",
              }
            : {
                state: "available",
                stats: {
                  memory_count: 33,
                  session_count: 9,
                  prompt_count: 4,
                  worker_enabled: true,
                  worker_queue_depth: 2,
                },
              }
        ),
        {
          status: options?.remoteUnavailable ? 503 : 200,
          headers: { "Content-Type": "application/json" },
        }
      )
    );
}

function mountStats() {
  return mount(MemoryStats, {
    props: {
      gatewayUrl: (path: string) => new URL(`http://localhost:3000/api${path}`).toString(),
      authHeaders: () => ({ Authorization: "Bearer token" }),
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("MemoryStats", () => {
  it("renders local and remote stats separately", async () => {
    queueDefaultRequests();

    const wrapper = mountStats();
    await flushPromises();

    expect(wrapper.text()).toContain("Local Memory");
    expect(wrapper.text()).toContain("Cerebro Memory");
    expect(wrapper.text()).toContain("Remote Memories");
    expect(wrapper.text()).toContain("33");
    expect(wrapper.text()).toContain("sqlite");
  });

  it("shows remote unreachable state without hiding local stats", async () => {
    queueDefaultRequests({ remoteUnavailable: true });

    const wrapper = mountStats();
    await flushPromises();

    expect(wrapper.text()).toContain("50");
    expect(wrapper.text()).toContain("Cerebro is currently unreachable.");
    expect(wrapper.text()).toContain("unreachable");
  });
});
