import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import GeneralSettings from "@/components/config/GeneralSettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("GeneralSettings", () => {
  it("emits provider and save updates", async () => {
    const wrapper = mount(GeneralSettings, {
      props: {
        modelValue: createAdminConfigForm({
          default_provider: "openrouter",
          default_model: "anthropic/model",
          api_url: "",
          default_temperature: "0.7",
          memory_backend: "sqlite",
        }),
        memoryBackendOptions: ["sqlite", "surreal"],
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("Default provider");
    expect(wrapper.text()).toContain("Default model");

    const inputs = wrapper.findAll("input");
    await inputs[0]?.setValue("openai");
    await inputs[1]?.setValue("gpt-5");
    await inputs[2]?.setValue("http://localhost:8787/api");
    await inputs[3]?.setValue("0.9");
    await wrapper.get("select").setValue("surreal");
    await wrapper.get("button").trigger("click");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(5);
    expect(updates?.[0]?.[0]).toEqual(expect.objectContaining({ default_provider: "openai" }));
    expect(updates?.[1]?.[0]).toEqual(expect.objectContaining({ default_model: "gpt-5" }));
    expect(updates?.[2]?.[0]).toEqual(
      expect.objectContaining({ api_url: "http://localhost:8787/api" })
    );
    expect(updates?.[3]?.[0]).toEqual(expect.objectContaining({ default_temperature: "0.9" }));
    expect(updates?.[4]?.[0]).toEqual(expect.objectContaining({ memory_backend: "surreal" }));
    expect(wrapper.emitted("save")).toHaveLength(1);
  });
});
