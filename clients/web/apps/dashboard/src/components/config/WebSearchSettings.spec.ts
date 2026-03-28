import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import WebSearchSettings from "@/components/config/WebSearchSettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("WebSearchSettings", () => {
  it("renders web search fields and emits updates", async () => {
    const initialForm = createAdminConfigForm({
      web_search_enabled: false,
      web_search_provider: "duckduckgo",
      web_search_max_results: "5",
      web_search_timeout_secs: "10",
      web_search_brave_api_key_mode: "unchanged",
      web_search_brave_api_key_value: "",
      web_search_has_brave_api_key: false,
    });

    const wrapper = mount(WebSearchSettings, {
      props: {
        modelValue: initialForm,
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("Web search");

    await wrapper.get('[data-testid="web_search_provider"]').setValue("brave");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(
      expect.objectContaining({
        web_search_provider: "brave",
        web_search_enabled: false,
        web_search_max_results: "5",
        web_search_timeout_secs: "10",
      })
    );

    await wrapper.get('button[data-testid="save"]').trigger("click");
    expect(wrapper.emitted("save")).toHaveLength(1);
  });
});
