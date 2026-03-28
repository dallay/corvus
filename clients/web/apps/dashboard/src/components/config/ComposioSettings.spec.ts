import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import ComposioSettings from "@/components/config/ComposioSettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("ComposioSettings", () => {
  it("renders composio fields and emits updates", async () => {
    const initialForm = createAdminConfigForm({
      composio_enabled: false,
      composio_entity_id: "default",
      composio_api_key_mode: "unchanged",
      composio_api_key_value: "",
      composio_has_api_key: false,
    });

    const wrapper = mount(ComposioSettings, {
      props: {
        modelValue: initialForm,
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("Composio");

    await wrapper.get('[data-testid="composio_entity_id"]').setValue("custom-entity");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(
      expect.objectContaining({
        composio_entity_id: "custom-entity",
        composio_enabled: false,
        composio_api_key_mode: "unchanged",
      })
    );

    await wrapper.get('button[data-testid="save"]').trigger("click");
    expect(wrapper.emitted("save")).toHaveLength(1);
  });

  it("shows password input when api key mode is replace", async () => {
    const wrapper = mount(ComposioSettings, {
      props: {
        modelValue: createAdminConfigForm({
          composio_api_key_mode: "replace",
          composio_api_key_value: "",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.find('[data-testid="composio_api_key_value"]').exists()).toBe(true);
  });

  it("hides password input when api key mode is clear", async () => {
    const wrapper = mount(ComposioSettings, {
      props: {
        modelValue: createAdminConfigForm({
          composio_api_key_mode: "clear",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.find('[data-testid="composio_api_key_value"]').exists()).toBe(false);
  });

  it("clears non-empty api key value when mode changes from replace to clear", async () => {
    const wrapper = mount(ComposioSettings, {
      props: {
        modelValue: createAdminConfigForm({
          composio_api_key_mode: "replace",
          composio_api_key_value: "existing-key-456",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    await wrapper.get('[data-testid="composio_api_key_mode"]').setValue("clear");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(
      expect.objectContaining({
        composio_api_key_mode: "clear",
        composio_api_key_value: "",
      })
    );
  });

  it("clears api key value when mode changes to clear", async () => {
    const wrapper = mount(ComposioSettings, {
      props: {
        modelValue: createAdminConfigForm({
          composio_api_key_mode: "unchanged",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    await wrapper.get('[data-testid="composio_api_key_mode"]').setValue("clear");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(
      expect.objectContaining({
        composio_api_key_mode: "clear",
        composio_api_key_value: "",
      })
    );
  });
});
