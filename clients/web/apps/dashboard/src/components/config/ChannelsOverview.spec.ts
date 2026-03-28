import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import ChannelsOverview from "@/components/config/ChannelsOverview.vue";
import { i18nConfig } from "@/i18n";

function mountComponent() {
  return mount(ChannelsOverview, {
    props: {
      gatewayUrl: "http://localhost:3000",
      bearerToken: "test-token",
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("ChannelsOverview", () => {
  it("renders channel list on successful fetch", async () => {
    const mockChannels = [
      { channel_type: "webhook", configured: true, config_summary: {} },
      { channel_type: "slack", configured: false, config_summary: {} },
    ];

    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ channels: mockChannels }),
      })
    );

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find('[data-testid="channel-webhook"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="channel-slack"]').exists()).toBe(true);

    const webhookItem = wrapper.find('[data-testid="channel-webhook"]');
    expect(webhookItem.find(".configured").exists()).toBe(true);
    expect(webhookItem.text()).toContain("Configured");

    const slackItem = wrapper.find('[data-testid="channel-slack"]');
    expect(slackItem.find(".not-configured").exists()).toBe(true);
    expect(slackItem.text()).toContain("Not configured");

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
