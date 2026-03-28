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
});
