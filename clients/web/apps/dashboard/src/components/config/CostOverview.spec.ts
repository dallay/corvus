import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import CostOverview from "@/components/config/CostOverview.vue";
import { i18nConfig } from "@/i18n";

function mountComponent() {
  return mount(CostOverview, {
    props: {
      gatewayUrl: "http://localhost:3000",
      bearerToken: "test-token",
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("CostOverview", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders cost data on successful fetch", async () => {
    const mockConfig = {
      config: {
        cost: {
          enabled: true,
          daily_limit_usd: 50,
          monthly_limit_usd: 1000,
          warn_at_percent: 80,
          allow_override: false,
        },
      },
    };

    const fetchSpy = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockConfig),
    });
    vi.stubGlobal("fetch", fetchSpy);

    const wrapper = mountComponent();
    await flushPromises();

    expect(fetchSpy).toHaveBeenCalledWith(
      expect.stringContaining("/web/admin/config"),
      expect.objectContaining({
        headers: { Authorization: "Bearer test-token" },
      })
    );

    expect(wrapper.find('[data-testid="cost-overview"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("$50.00");
    expect(wrapper.text()).toContain("$1,000.00");
    expect(wrapper.text()).toContain("80%");
  });

  it("shows error on fetch failure", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("Network error")));

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find(".error").exists()).toBe(true);
    expect(wrapper.text()).toContain("Network error");
  });

  it("shows error when cost data is missing from response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ config: {} }),
      })
    );

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find(".error").exists()).toBe(true);
    expect(wrapper.text()).toContain("Cost data not available");
  });
});
