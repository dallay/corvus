import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import SecuritySettings from "@/components/config/SecuritySettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("SecuritySettings", () => {
  it("emits autonomy, identity, and save updates", async () => {
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
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("Identity format");
    expect(wrapper.text()).toContain("Identity AIEOS path");
    expect(wrapper.text()).toContain("Require approval for medium risk");
    expect(wrapper.text()).toContain("Block high risk commands");
    expect(wrapper.text()).toContain("Auto-approve commands (comma-separated)");
    expect(wrapper.text()).toContain("Always ask commands (comma-separated)");

    expect(wrapper.find('[data-testid="autonomy-require-approval-medium-risk"]').exists()).toBe(
      true
    );
    expect(wrapper.find('[data-testid="autonomy-block-high-risk-commands"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="autonomy-auto-approve"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="autonomy-always-ask"]').exists()).toBe(true);

    await wrapper.get("select").setValue("full");

    const inputs = wrapper.findAll('input:not([type="checkbox"])');
    await inputs[0]?.setValue("30");
    await inputs[1]?.setValue("900");
    await inputs[2]?.setValue("aieos");
    await inputs[3]?.setValue("/tmp/identity.json");

    const checkbox = wrapper.get('input[type="checkbox"]');
    await checkbox.setValue(false);
    await wrapper.get("button").trigger("click");

    expect(wrapper.emitted("update:modelValue")).toEqual([
      [expect.objectContaining({ autonomy_level: "full" })],
      [expect.objectContaining({ autonomy_max_actions_per_hour: "30" })],
      [expect.objectContaining({ autonomy_max_cost_per_day_cents: "900" })],
      [expect.objectContaining({ identity_format: "aieos" })],
      [expect.objectContaining({ identity_aieos_path: "/tmp/identity.json" })],
      [expect.objectContaining({ autonomy_workspace_only: false })],
    ]);
    expect(wrapper.emitted("save")).toHaveLength(1);
  });
});
