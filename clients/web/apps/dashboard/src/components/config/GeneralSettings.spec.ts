import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import GeneralSettings from "@/components/config/GeneralSettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("GeneralSettings", () => {
  it("emits provider and save updates", async () => {
    const initialForm = createAdminConfigForm({
      default_provider: "openrouter",
      default_model: "anthropic/model",
      api_url: "",
      default_temperature: "0.7",
      memory_backend: "sqlite",
    });

    const wrapper = mount(GeneralSettings, {
      props: {
        modelValue: initialForm,
        memoryBackendOptions: ["sqlite", "lucid"],
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("Default provider");
    expect(wrapper.text()).toContain("Default model");

    // Use stable data-testid selectors instead of positional indexing
    await wrapper.get('[data-testid="default_provider"]').setValue("openai");
    await wrapper.get('[data-testid="default_model"]').setValue("gpt-5");
    await wrapper.get('[data-testid="api_url"]').setValue("http://localhost:8787/api");
    await wrapper.get('[data-testid="default_temperature"]').setValue("0.9");
    await wrapper.get('select[data-testid="memory_backend"]').setValue("lucid");
    await wrapper.get('button[data-testid="save"]').trigger("click");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(5);

    // Each emit must include the changed field AND companion fields from the form.
    // objectContaining is used (not toEqual) for partial matching — this is stricter
    // than the original single-field checks while remaining compatible with the
    // component's reactive model (parent doesn't update modelValue between emits).
    expect(updates?.[0]?.[0]).toEqual(
      expect.objectContaining({
        default_provider: "openai",
        default_model: "anthropic/model",
        api_url: "",
        default_temperature: "0.7",
        memory_backend: "sqlite",
      })
    );
    expect(updates?.[1]?.[0]).toEqual(
      expect.objectContaining({
        default_provider: "openrouter",
        default_model: "gpt-5",
        api_url: "",
        default_temperature: "0.7",
        memory_backend: "sqlite",
      })
    );
    expect(updates?.[2]?.[0]).toEqual(
      expect.objectContaining({
        default_provider: "openrouter",
        default_model: "anthropic/model",
        api_url: "http://localhost:8787/api",
        default_temperature: "0.7",
        memory_backend: "sqlite",
      })
    );
    expect(updates?.[3]?.[0]).toEqual(
      expect.objectContaining({
        default_provider: "openrouter",
        default_model: "anthropic/model",
        api_url: "",
        default_temperature: "0.9",
        memory_backend: "sqlite",
      })
    );
    expect(updates?.[4]?.[0]).toEqual(
      expect.objectContaining({
        default_provider: "openrouter",
        default_model: "anthropic/model",
        api_url: "",
        default_temperature: "0.7",
        memory_backend: "lucid",
      })
    );
    expect(wrapper.emitted("save")).toHaveLength(1);
  });
});
