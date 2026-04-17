import { mount } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import WebhookSettings from "@/components/config/WebhookSettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

const mountedWrappers = new Set<ReturnType<typeof mount>>();

function mountWebhookSettings(modelValue = createAdminConfigForm()) {
  const wrapper = mount(WebhookSettings, {
    attachTo: document.body,
    props: {
      modelValue,
      disabled: false,
      saving: false,
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });

  mountedWrappers.add(wrapper);
  return wrapper;
}

afterEach(() => {
  for (const wrapper of mountedWrappers) {
    wrapper.unmount();
  }
  mountedWrappers.clear();
  document.body.innerHTML = "";
});

describe("WebhookSettings", () => {
  it("shows local validation feedback for replace mode", async () => {
    const wrapper = mountWebhookSettings(
      createAdminConfigForm({
        webhook_enabled: true,
        webhook_port: "3000",
        webhook_secret_mode: "replace",
        webhook_secret_value: "",
        webhook_secret_exists: false,
      })
    );

    expect(wrapper.text()).toContain("cannot be empty");
    expect(wrapper.get('[role="alert"]').text()).toContain(
      "Please fix the following error before saving"
    );
    expect(wrapper.get('input[type="password"]').attributes("aria-describedby")).toBe(
      "webhook-secret-help webhook-secret-error"
    );
    expect(wrapper.get('input[type="password"]').attributes("aria-invalid")).toBe("true");
    expect(wrapper.text()).toContain("password managers or secure vault tools");
    await wrapper.get(".actions button").trigger("click");
    expect(document.activeElement).toBe(wrapper.get('[role="alert"]').element);
    expect(wrapper.emitted("save")).toBeFalsy();
  });

  it("moves focus to the invalid field from the error summary", async () => {
    const wrapper = mountWebhookSettings(
      createAdminConfigForm({
        webhook_enabled: true,
        webhook_port: "3000",
        webhook_secret_mode: "replace",
        webhook_secret_value: "",
        webhook_secret_exists: false,
      })
    );

    await wrapper.get(".error-summary-link").trigger("click");

    expect(document.activeElement).toBe(wrapper.get('input[type="password"]').element);
  });

  it("emits webhook field changes and saves when valid", async () => {
    const wrapper = mountWebhookSettings(
      createAdminConfigForm({
        webhook_enabled: false,
        webhook_port: "3001",
        webhook_secret_mode: "unchanged",
        webhook_secret_value: "",
        webhook_secret_exists: true,
      })
    );

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
    await wrapper.get(".actions button").trigger("click");

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
