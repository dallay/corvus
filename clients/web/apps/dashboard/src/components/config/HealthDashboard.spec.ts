import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import HealthDashboard from "@/components/config/HealthDashboard.vue";
import { i18nConfig } from "@/i18n";

function mountComponent() {
  return mount(HealthDashboard, {
    props: {
      gatewayUrl: "http://localhost:3000",
      bearerToken: "test-token",
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("HealthDashboard", () => {
  it("renders health data on successful fetch", async () => {
    const mockHealth = {
      pid: 1234,
      updated_at: "2026-01-01T00:00:00Z",
      uptime_seconds: 90061,
      components: {
        gateway: {
          status: "ok",
          updated_at: "2026-01-01T00:00:00Z",
          last_ok: "2026-01-01T00:00:00Z",
          last_error: null,
          restart_count: 0,
        },
        memory: {
          status: "error",
          updated_at: "2026-01-01T00:00:00Z",
          last_ok: null,
          last_error: "connection lost",
          restart_count: 2,
        },
      },
    };

    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ health: mockHealth }),
      })
    );

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find('[data-testid="health-gateway"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="health-memory"]').exists()).toBe(true);

    const gatewayItem = wrapper.find('[data-testid="health-gateway"]');
    expect(gatewayItem.find(".ok").exists()).toBe(true);

    const memoryItem = wrapper.find('[data-testid="health-memory"]');
    expect(memoryItem.find(".error").exists()).toBe(true);
    expect(memoryItem.text()).toContain("Restarts: 2");

    expect(wrapper.text()).toContain("1d 1h 1m");

    vi.unstubAllGlobals();
  });

  it("shows error on fetch failure", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("Network error")));

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find(".error").exists()).toBe(true);
    expect(wrapper.text()).toContain("Network error");

    vi.unstubAllGlobals();
  });
});
