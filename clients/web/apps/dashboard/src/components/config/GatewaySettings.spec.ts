import { mount } from "@vue/test-utils";
import { createI18n } from "vue-i18n";
import { describe, expect, it } from "vitest";

import GatewaySettings from "@/components/config/GatewaySettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("GatewaySettings", () => {
  it("renders server controls", () => {
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
        plugins: [createI18n(i18nConfig)],
      },
    });

    expect(wrapper.text()).toContain("Gateway");
    expect(wrapper.text()).toContain("Gateway host");
  });
});
