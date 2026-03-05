import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import SecuritySettings from "@/components/config/SecuritySettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("SecuritySettings", () => {
  it("renders identity-focused controls", () => {
    const wrapper = mount(SecuritySettings, {
      props: {
        modelValue: createAdminConfigForm({
          autonomy_level: "supervised",
          autonomy_workspace_only: true,
          autonomy_max_actions_per_hour: "20",
          autonomy_max_cost_per_day_cents: "500",
          identity_format: "openclaw",
          identity_aieos_path: "identity.json",
        }),
        autonomyLevelOptions: ["readonly", "supervised", "full"],
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n(i18nConfig)],
      },
    });

    expect(wrapper.text()).toContain("Formato de identidad");
    expect(wrapper.text()).toContain("Ruta AIEOS de identidad");
  });
});
