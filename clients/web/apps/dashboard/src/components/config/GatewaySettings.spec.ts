import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import GatewaySettings from "@/components/config/GatewaySettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("GatewaySettings", () => {
  it("emits gateway field updates and save actions", async () => {
    const wrapper = mount(GatewaySettings, {
      props: {
        modelValue: createAdminConfigForm({
          gateway_port: "3000",
          gateway_host: "127.0.0.1",
          gateway_require_pairing: true,
          gateway_allow_public_bind: false,
          gateway_pair_rate_limit_per_minute: "10",
          gateway_webhook_rate_limit_per_minute: "60",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("Gateway");
    expect(wrapper.text()).toContain("Gateway host");

    const checkboxes = wrapper.findAll('input[type="checkbox"]');
    const numberInputs = wrapper.findAll('input[type="number"]');
    const textInput = wrapper.get('input[type="text"]');

    await numberInputs[0]?.setValue("4000");
    await textInput.setValue("0.0.0.0");
    await numberInputs[1]?.setValue("20");
    await numberInputs[2]?.setValue("120");
    await checkboxes[0]?.setValue(false);
    await checkboxes[1]?.setValue(true);
    await wrapper.get("button").trigger("click");

    expect(wrapper.emitted("update:modelValue")).toEqual([
      [expect.objectContaining({ gateway_port: "4000" })],
      [expect.objectContaining({ gateway_host: "0.0.0.0" })],
      [expect.objectContaining({ gateway_pair_rate_limit_per_minute: "20" })],
      [expect.objectContaining({ gateway_webhook_rate_limit_per_minute: "120" })],
      [expect.objectContaining({ gateway_require_pairing: false })],
      [expect.objectContaining({ gateway_allow_public_bind: true })],
    ]);
    expect(wrapper.emitted("save")).toHaveLength(1);
  });
});
