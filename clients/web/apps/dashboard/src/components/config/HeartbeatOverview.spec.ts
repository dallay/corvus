import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import HeartbeatOverview from "@/components/config/HeartbeatOverview.vue";
import { i18nConfig } from "@/i18n";

function mountComponent() {
  return mount(HeartbeatOverview, {
    props: {
      gatewayUrl: "http://localhost:3000",
      bearerToken: "test-token",
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("HeartbeatOverview", () => {
  it("renders heartbeat data on successful fetch", async () => {
    const mockConfig = {
      config: {
        heartbeat: {
          enabled: true,
          interval_minutes: 5,
        },
      },
    };

    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockConfig),
      })
    );

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find('[data-testid="heartbeat-overview"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("5 min");

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
