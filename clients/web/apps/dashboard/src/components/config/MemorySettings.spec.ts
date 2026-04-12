import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import MemorySettings from "@/components/config/MemorySettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("MemorySettings", () => {
  it("renders memory fields and emits updates", async () => {
    const initialForm = createAdminConfigForm({
      memory_cerebro_endpoint: "",
      memory_cerebro_timeout_ms: "5000",
      memory_cerebro_allow_insecure_loopback: false,
      memory_cerebro_auth_token_mode: "unchanged",
      memory_cerebro_auth_token_value: "",
      memory_cerebro_has_auth_token: false,
    });

    const wrapper = mount(MemorySettings, {
      props: {
        modelValue: initialForm,
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("Memory");

    await wrapper.get('[data-testid="memory_cerebro_endpoint"]').setValue("http://localhost:9090");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(
      expect.objectContaining({
        memory_cerebro_endpoint: "http://localhost:9090",
        memory_cerebro_timeout_ms: "5000",
        memory_cerebro_allow_insecure_loopback: false,
      })
    );

    await wrapper.get('button[data-testid="save"]').trigger("click");
    expect(wrapper.emitted("save")).toHaveLength(1);
  });

  it("shows password input when auth token mode is replace", async () => {
    const wrapper = mount(MemorySettings, {
      props: {
        modelValue: createAdminConfigForm({
          memory_cerebro_auth_token_mode: "replace",
          memory_cerebro_auth_token_value: "",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.find('[data-testid="memory_cerebro_auth_token_value"]').exists()).toBe(true);
    expect(
      wrapper.get('[data-testid="memory_cerebro_auth_token_value"]').attributes("aria-describedby")
    ).toBe("memory-cerebro-auth-token-help");
    expect(wrapper.text()).toContain("password managers or secure vault tools");
  });

  it("hides password input when auth token mode is clear", async () => {
    const wrapper = mount(MemorySettings, {
      props: {
        modelValue: createAdminConfigForm({
          memory_cerebro_auth_token_mode: "clear",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.find('[data-testid="memory_cerebro_auth_token_value"]').exists()).toBe(false);
  });

  it("clears auth token value when mode changes to clear", async () => {
    const wrapper = mount(MemorySettings, {
      props: {
        modelValue: createAdminConfigForm({
          memory_cerebro_auth_token_mode: "replace",
          memory_cerebro_auth_token_value: "secret",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    await wrapper.get('[data-testid="memory_cerebro_auth_token_mode"]').setValue("clear");

    const updates = wrapper.emitted("update:modelValue");
    expect(updates).toHaveLength(1);
    expect(updates?.[0]?.[0]).toEqual(
      expect.objectContaining({
        memory_cerebro_auth_token_mode: "clear",
        memory_cerebro_auth_token_value: "",
      })
    );
  });
});
