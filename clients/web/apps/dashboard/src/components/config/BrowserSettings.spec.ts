import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import BrowserSettings from "@/components/config/BrowserSettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("BrowserSettings", () => {
  it("renders browser fields and emits updates", async () => {
    const initialForm = createAdminConfigForm({
      browser_computer_use_api_key_mode: "unchanged",
      browser_computer_use_api_key_value: "",
      browser_has_computer_use_api_key: false,
    });

    const wrapper = mount(BrowserSettings, {
      props: {
        modelValue: initialForm,
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("Browser");

    await wrapper
      .get('select[data-testid="browser_computer_use_api_key_mode"]')
      .setValue("replace");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(
      expect.objectContaining({
        browser_computer_use_api_key_mode: "replace",
      })
    );

    await wrapper.get('button[data-testid="save"]').trigger("click");
    expect(wrapper.emitted("save")).toHaveLength(1);
  });

  it("shows password input when computer use api key mode is replace", () => {
    const wrapper = mount(BrowserSettings, {
      props: {
        modelValue: createAdminConfigForm({
          browser_computer_use_api_key_mode: "replace",
          browser_computer_use_api_key_value: "",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.find('[data-testid="browser_computer_use_api_key_value"]').exists()).toBe(true);
  });

  it("hides password input when computer use api key mode is clear", () => {
    const wrapper = mount(BrowserSettings, {
      props: {
        modelValue: createAdminConfigForm({
          browser_computer_use_api_key_mode: "clear",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.find('[data-testid="browser_computer_use_api_key_value"]').exists()).toBe(false);
  });

  it("clears api key value when switching from replace to clear", async () => {
    const wrapper = mount(BrowserSettings, {
      props: {
        modelValue: createAdminConfigForm({
          browser_computer_use_api_key_mode: "replace",
          browser_computer_use_api_key_value: "secret-key-123",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    await wrapper.get('select[data-testid="browser_computer_use_api_key_mode"]').setValue("clear");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(
      expect.objectContaining({
        browser_computer_use_api_key_mode: "clear",
        browser_computer_use_api_key_value: "",
      })
    );
  });

  it("hides password input and does not leak secret when mode is unchanged", () => {
    const wrapper = mount(BrowserSettings, {
      props: {
        modelValue: createAdminConfigForm({
          browser_computer_use_api_key_mode: "unchanged",
          browser_has_computer_use_api_key: true,
          browser_computer_use_api_key_value: "supersecret",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.find('[data-testid="browser_computer_use_api_key_value"]').exists()).toBe(false);
    expect(wrapper.text()).not.toContain("supersecret");
  });
});
