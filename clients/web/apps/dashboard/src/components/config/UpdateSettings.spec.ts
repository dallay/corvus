import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import UpdateSettings from "@/components/config/UpdateSettings.vue";
import { i18nConfig } from "@/i18n";
import type { AdminConfigView } from "@/types/admin-config";

describe("UpdateSettings", () => {
  it("renders update status", () => {
    const config: AdminConfigView = {
      updates: {
        enabled: true,
        auto_install_enabled: false,
        channel_visibility_enabled: true,
        cli_startup_notice_enabled: true,
        restart_policy: "graceful",
        status: {
          current_version: "1.2.3",
          latest_version: "1.3.0",
          update_available: true,
          effective_install_method: "cargo",
          install_method_source: "detected",
        },
      },
    };

    const wrapper = mount(UpdateSettings, {
      props: {
        config,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("1.2.3");
    expect(wrapper.text()).toContain("1.3.0");
    expect(wrapper.find('button[data-testid="save"]').exists()).toBe(false);
  });
});
