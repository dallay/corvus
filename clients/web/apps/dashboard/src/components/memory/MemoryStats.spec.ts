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
  const remoteToolState = options?.remoteUnavailable ? "unreachable" : "available";

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
            mem_search: { state: remoteToolState },
            mem_get_observation: { state: remoteToolState },
            mem_timeline: { state: remoteToolState },
            mem_stats: { state: remoteToolState },
            mem_save: { state: remoteToolState },
            mem_update: { state: remoteToolState },
            mem_delete: { state: remoteToolState },
            mem_save_prompt: { state: remoteToolState },
            mem_session_start: { state: remoteToolState },
            mem_session_end: { state: remoteToolState },
            mem_session_summary: { state: remoteToolState },
            mem_context: { state: remoteToolState },
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

  it("emits a local category drill-in event without touching Cerebro cards", async () => {
    queueDefaultRequests();

    const wrapper = mountStats();
    await flushPromises();

    await wrapper.findAll("button.category-item")[0]?.trigger("click");

    expect(wrapper.emitted("select-category")).toEqual([["Core"]]);
    expect(wrapper.text()).toContain("Cerebro Memory");
  });
});
