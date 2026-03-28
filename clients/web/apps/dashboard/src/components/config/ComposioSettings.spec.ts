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
});
