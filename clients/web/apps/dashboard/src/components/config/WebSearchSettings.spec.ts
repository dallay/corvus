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

  it("shows password input when brave api key mode is replace", async () => {
    const wrapper = mount(WebSearchSettings, {
      props: {
        modelValue: createAdminConfigForm({
          web_search_brave_api_key_mode: "replace",
          web_search_brave_api_key_value: "",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.find('[data-testid="web_search_brave_api_key_value"]').exists()).toBe(true);
    expect(
      wrapper.get('[data-testid="web_search_brave_api_key_value"]').attributes("aria-describedby")
    ).toBe("web-search-brave-api-key-help");
    expect(wrapper.text()).toContain("password managers or secure vault tools");
  });

  it("hides password input when brave api key mode is clear", async () => {
    const wrapper = mount(WebSearchSettings, {
      props: {
        modelValue: createAdminConfigForm({
          web_search_brave_api_key_mode: "clear",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.find('[data-testid="web_search_brave_api_key_value"]').exists()).toBe(false);
  });

  it("hides password input when brave api key mode is unchanged", async () => {
    const wrapper = mount(WebSearchSettings, {
      props: {
        modelValue: createAdminConfigForm({
          web_search_brave_api_key_mode: "unchanged",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.find('[data-testid="web_search_brave_api_key_value"]').exists()).toBe(false);
  });

  it("clears brave api key value when switching from replace to clear", async () => {
    const wrapper = mount(WebSearchSettings, {
      props: {
        modelValue: createAdminConfigForm({
          web_search_brave_api_key_mode: "replace",
          web_search_brave_api_key_value: "brave-key-789",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    await wrapper.get('[data-testid="web_search_brave_api_key_mode"]').setValue("clear");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(
      expect.objectContaining({
        web_search_brave_api_key_mode: "clear",
        web_search_brave_api_key_value: "",
      })
    );
  });

  it("clears brave api key value when switching from replace to unchanged", async () => {
    const wrapper = mount(WebSearchSettings, {
      props: {
        modelValue: createAdminConfigForm({
          web_search_brave_api_key_mode: "replace",
          web_search_brave_api_key_value: "key123",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    await wrapper.get('[data-testid="web_search_brave_api_key_mode"]').setValue("unchanged");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(
      expect.objectContaining({
        web_search_brave_api_key_mode: "unchanged",
        web_search_brave_api_key_value: "",
      })
    );
  });

  it("clamps max_results above 10 down to 10", async () => {
    const wrapper = mount(WebSearchSettings, {
      props: {
        modelValue: createAdminConfigForm({ web_search_max_results: "5" }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    await wrapper.get('[data-testid="web_search_max_results"]').setValue("15");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(expect.objectContaining({ web_search_max_results: "10" }));
  });

  it("clamps max_results below 1 up to 1", async () => {
    const wrapper = mount(WebSearchSettings, {
      props: {
        modelValue: createAdminConfigForm({ web_search_max_results: "5" }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    await wrapper.get('[data-testid="web_search_max_results"]').setValue("0");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(expect.objectContaining({ web_search_max_results: "1" }));
  });

  it("clamps timeout_secs below 1 up to 1", async () => {
    const wrapper = mount(WebSearchSettings, {
      props: {
        modelValue: createAdminConfigForm({ web_search_timeout_secs: "10" }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    await wrapper.get('[data-testid="web_search_timeout_secs"]').setValue("0");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(expect.objectContaining({ web_search_timeout_secs: "1" }));
  });

  it("disables all inputs when disabled prop is true", () => {
    const wrapper = mount(WebSearchSettings, {
      props: {
        modelValue: createAdminConfigForm(),
        disabled: true,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    const checkbox = wrapper.get('[data-testid="web_search_enabled"]');
    expect((checkbox.element as HTMLInputElement).disabled).toBe(true);

    const provider = wrapper.get('[data-testid="web_search_provider"]');
    expect((provider.element as HTMLInputElement).disabled).toBe(true);

    const maxResults = wrapper.get('[data-testid="web_search_max_results"]');
    expect((maxResults.element as HTMLInputElement).disabled).toBe(true);

    const timeout = wrapper.get('[data-testid="web_search_timeout_secs"]');
    expect((timeout.element as HTMLInputElement).disabled).toBe(true);

    const select = wrapper.get('[data-testid="web_search_brave_api_key_mode"]');
    expect((select.element as HTMLSelectElement).disabled).toBe(true);

    const saveBtn = wrapper.get('button[data-testid="save"]');
    expect((saveBtn.element as HTMLButtonElement).disabled).toBe(true);
  });

  it("accepts input in password field when mode is replace", async () => {
    const wrapper = mount(WebSearchSettings, {
      props: {
        modelValue: createAdminConfigForm({
          web_search_brave_api_key_mode: "replace",
          web_search_brave_api_key_value: "",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    const input = wrapper.get('[data-testid="web_search_brave_api_key_value"]');
    expect(input.exists()).toBe(true);
    await input.setValue("new-secret-key");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(
      expect.objectContaining({ web_search_brave_api_key_value: "new-secret-key" })
    );
  });

  it("ignores invalid secret mode values", async () => {
    const wrapper = mount(WebSearchSettings, {
      props: {
        modelValue: createAdminConfigForm(),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    const select = wrapper.get('[data-testid="web_search_brave_api_key_mode"]');
    (select.element as HTMLSelectElement).value = "bogus";
    await select.trigger("change");

    expect(wrapper.emitted("update:modelValue")).toBeUndefined();
  });
});
