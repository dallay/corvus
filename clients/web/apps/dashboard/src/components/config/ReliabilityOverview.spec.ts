import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import ReliabilityOverview from "@/components/config/ReliabilityOverview.vue";
import { i18nConfig } from "@/i18n";

function mountComponent() {
  return mount(ReliabilityOverview, {
    props: {
      gatewayUrl: "http://localhost:3000",
      bearerToken: "test-token",
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("ReliabilityOverview", () => {
  it("renders reliability data on successful fetch", async () => {
    const mockConfig = {
      config: {
        reliability: {
          provider_retries: 3,
          provider_backoff_ms: 1000,
          fallback_providers: ["openai", "anthropic"],
          model_fallbacks: { "gpt-4": ["gpt-3.5-turbo"] },
          channel_initial_backoff_secs: 5,
          channel_max_backoff_secs: 300,
          scheduler_poll_secs: 10,
          scheduler_retries: 2,
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

    expect(wrapper.find('[data-testid="reliability-overview"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("3");
    expect(wrapper.text()).toContain("1000ms");
    expect(wrapper.text()).toContain("openai, anthropic");

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
