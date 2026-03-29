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

function mockStatsResponse(stats: Record<string, unknown>) {
  fetchMock.mockResolvedValueOnce(
    new Response(JSON.stringify(stats), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    })
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
  it("renders all stats fields", async () => {
    mockStatsResponse({
      total_entries: 50,
      by_category: { Core: 20, Conversation: 15, Daily: 10, Custom: 5 },
      total_sessions: 8,
      active_sessions: 3,
      backend: "sqlite",
      cerebro_configured: false,
    });

    const wrapper = mountStats();
    await flushPromises();

    expect(wrapper.text()).toContain("50");
    expect(wrapper.text()).toContain("8");
    expect(wrapper.text()).toContain("3");
    expect(wrapper.text()).toContain("sqlite");
    expect(wrapper.text()).toContain("Total Entries");
    expect(wrapper.text()).toContain("Total Sessions");
    expect(wrapper.text()).toContain("Active Sessions");
    expect(wrapper.text()).toContain("Backend");
  });

  it("shows Cerebro as Not configured when cerebro_configured is false", async () => {
    mockStatsResponse({
      total_entries: 10,
      by_category: {},
      total_sessions: 2,
      active_sessions: 1,
      backend: "sqlite",
      cerebro_configured: false,
    });

    const wrapper = mountStats();
    await flushPromises();

    expect(wrapper.text()).toContain("Not configured");
    expect(wrapper.find(".indicator-off").exists()).toBe(true);
  });

  it("shows Cerebro as Configured when cerebro_configured is true", async () => {
    mockStatsResponse({
      total_entries: 10,
      by_category: {},
      total_sessions: 2,
      active_sessions: 1,
      backend: "sqlite",
      cerebro_configured: true,
    });

    const wrapper = mountStats();
    await flushPromises();

    expect(wrapper.text()).toContain("Configured");
    expect(wrapper.find(".indicator-ok").exists()).toBe(true);
  });

  it("displays category breakdown when categories exist", async () => {
    mockStatsResponse({
      total_entries: 50,
      by_category: { Core: 20, Conversation: 15, Daily: 10, Custom: 5 },
      total_sessions: 8,
      active_sessions: 3,
      backend: "sqlite",
      cerebro_configured: false,
    });

    const wrapper = mountStats();
    await flushPromises();

    expect(wrapper.text()).toContain("By Category");
    expect(wrapper.text()).toContain("Core");
    expect(wrapper.text()).toContain("20");
    expect(wrapper.text()).toContain("Conversation");
    expect(wrapper.text()).toContain("15");
    expect(wrapper.text()).toContain("Daily");
    expect(wrapper.text()).toContain("10");
    expect(wrapper.text()).toContain("Custom");
    expect(wrapper.text()).toContain("5");
  });

  it("does not show category breakdown when by_category is empty", async () => {
    mockStatsResponse({
      total_entries: 0,
      by_category: {},
      total_sessions: 0,
      active_sessions: 0,
      backend: "sqlite",
      cerebro_configured: false,
    });

    const wrapper = mountStats();
    await flushPromises();

    expect(wrapper.find(".category-breakdown").exists()).toBe(false);
    expect(wrapper.text()).toContain("0");
  });
});
