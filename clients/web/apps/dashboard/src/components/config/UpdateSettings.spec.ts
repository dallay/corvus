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

  it("renders fallback values when updates data is null", () => {
    const config: AdminConfigView = {};

    const wrapper = mount(UpdateSettings, {
      props: { config },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.get('[data-testid="updates_current_version"]').text()).toBe("—");
    expect(wrapper.get('[data-testid="updates_restart_policy"]').text()).toBe("—");
    expect(wrapper.get('[data-testid="updates_effective_install_method"]').text()).toBe("—");
  });

  it("renders fallback values when status is missing but updates exists", () => {
    const config: AdminConfigView = {
      updates: {
        enabled: true,
        auto_install_enabled: false,
        channel_visibility_enabled: false,
        cli_startup_notice_enabled: false,
        restart_policy: "immediate",
      },
    };

    const wrapper = mount(UpdateSettings, {
      props: { config },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.get('[data-testid="updates_current_version"]').text()).toBe("—");
    expect(wrapper.get('[data-testid="updates_restart_policy"]').text()).toBe("immediate");
  });

  it("renders yes for update_available when true", () => {
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
      props: { config },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.get('[data-testid="updates_update_available"]').text()).toContain("Yes");
  });

  it("renders no for update_available when false", () => {
    const config: AdminConfigView = {
      updates: {
        enabled: true,
        auto_install_enabled: true,
        channel_visibility_enabled: true,
        cli_startup_notice_enabled: true,
        restart_policy: "graceful",
        status: {
          current_version: "2.0.0",
          latest_version: "2.0.0",
          update_available: false,
          effective_install_method: "brew",
          install_method_source: "detected",
        },
      },
    };

    const wrapper = mount(UpdateSettings, {
      props: { config },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.get('[data-testid="updates_update_available"]').text()).toContain("No");
    expect(wrapper.get('[data-testid="updates_auto_install_enabled"]').text()).toContain("Yes");
  });
});
