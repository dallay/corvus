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

  it("emits webhook field changes and saves when valid", async () => {
    const wrapper = mount(WebhookSettings, {
      props: {
        modelValue: createAdminConfigForm({
          webhook_enabled: false,
          webhook_port: "3001",
          webhook_secret_mode: "unchanged",
          webhook_secret_value: "",
          webhook_secret_exists: true,
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    await wrapper.get('input[type="checkbox"]').setValue(true);
    await wrapper.get("select").setValue("replace");
    await wrapper.setProps({
      modelValue: createAdminConfigForm({
        webhook_enabled: false,
        webhook_port: "3001",
        webhook_secret_mode: "replace",
        webhook_secret_value: "",
        webhook_secret_exists: true,
      }),
    });

    const passwordInput = wrapper.get('input[type="password"]');
    const numberInput = wrapper.get('input[type="number"]');
    await numberInput.setValue("3010");
    await passwordInput.setValue("top-secret");
    await wrapper.setProps({
      modelValue: createAdminConfigForm({
        webhook_enabled: true,
        webhook_port: "3010",
        webhook_secret_mode: "replace",
        webhook_secret_value: "top-secret",
        webhook_secret_exists: true,
      }),
    });
    await wrapper.get("button").trigger("click");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(4);
    expect(updates?.[0]?.[0]).toEqual(expect.objectContaining({ webhook_enabled: true }));
    expect(updates?.[1]?.[0]).toEqual(expect.objectContaining({ webhook_secret_mode: "replace" }));
    expect(updates?.[2]?.[0]).toEqual(expect.objectContaining({ webhook_port: "3010" }));
    expect(updates?.[3]?.[0]).toEqual(
      expect.objectContaining({ webhook_secret_value: "top-secret" })
    );
    expect(wrapper.text()).toContain("Current secret: configured");
    expect(wrapper.emitted("save")).toHaveLength(1);
  });
});
