import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import WebhookSettings from "@/components/config/WebhookSettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("WebhookSettings", () => {
  it("shows local validation feedback for replace mode", async () => {
    const wrapper = mount(WebhookSettings, {
      props: {
        modelValue: createAdminConfigForm({
          webhook_enabled: true,
          webhook_port: "3000",
          webhook_secret_mode: "replace",
          webhook_secret_value: "",
          webhook_secret_exists: false,
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("cannot be empty");
    await wrapper.get("button").trigger("click");
    expect(wrapper.emitted("save")).toBeFalsy();
  });
});
