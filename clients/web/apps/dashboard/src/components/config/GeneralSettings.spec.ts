import { mount } from "@vue/test-utils";
import { createI18n } from "vue-i18n";
import { describe, expect, it } from "vitest";

import { i18nConfig } from "@/i18n";
import GeneralSettings from "@/components/config/GeneralSettings.vue";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("GeneralSettings", () => {
  it("renders provider-focused inputs", () => {
    const wrapper = mount(GeneralSettings, {
      props: {
        modelValue: createAdminConfigForm({
          default_provider: "openrouter",
          default_model: "anthropic/model",
          api_url: "",
          default_temperature: "0.7",
          memory_backend: "sqlite",
        }),
        memoryBackendOptions: ["sqlite"],
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n(i18nConfig)],
      },
    });

    expect(wrapper.text()).toContain("Provider por defecto");
    expect(wrapper.text()).toContain("Modelo por defecto");
  });
});
