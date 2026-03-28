import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import TunnelOverview from "@/components/config/TunnelOverview.vue";
import { i18nConfig } from "@/i18n";

function mountComponent() {
  return mount(TunnelOverview, {
    props: {
      gatewayUrl: "http://localhost:3000",
      bearerToken: "test-token",
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("TunnelOverview", () => {
  it("renders tunnel data on successful fetch", async () => {
    const mockConfig = {
      config: {
        tunnel: {
          provider: "cloudflare",
          has_cloudflare_token: true,
          tailscale_funnel: null,
          tailscale_hostname: null,
          has_ngrok_auth_token: false,
          ngrok_domain: null,
          custom_health_url: null,
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

    expect(wrapper.find('[data-testid="tunnel-overview"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("cloudflare");

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
