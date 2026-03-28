import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import SchedulerStatus from "@/components/config/SchedulerStatus.vue";
import { i18nConfig } from "@/i18n";

function mountComponent() {
  return mount(SchedulerStatus, {
    props: {
      gatewayUrl: "http://localhost:3000",
      bearerToken: "test-token",
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("SchedulerStatus", () => {
  it("renders scheduler status on successful fetch", async () => {
    const mockScheduler = {
      enabled: true,
      max_tasks: 64,
      max_concurrent: 4,
      task_count: 0,
    };

    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ scheduler: mockScheduler }),
      })
    );

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find('[data-testid="scheduler-status"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("64");
    expect(wrapper.text()).toContain("4");
    expect(wrapper.text()).toContain("Not available");
    expect(wrapper.text()).toContain("Yes");

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

  it("renders disabled state correctly", async () => {
    const mockScheduler = {
      enabled: false,
      max_tasks: 32,
      max_concurrent: 2,
      task_count: 0,
    };

    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ scheduler: mockScheduler }),
      })
    );

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.text()).toContain("No");
    expect(wrapper.find(".not-configured").exists()).toBe(true);

    vi.unstubAllGlobals();
  });
});
