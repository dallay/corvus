import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import ObservabilitySettings from "@/components/config/ObservabilitySettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("ObservabilitySettings", () => {
  it("emits field updates and save actions", async () => {
    const wrapper = mount(ObservabilitySettings, {
      props: {
        modelValue: createAdminConfigForm({
          observability_backend: "none",
          otel_endpoint: "",
          otel_service_name: "",
        }),
        observabilityBackendOptions: ["none", "otel"],
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("Observability backend");

    await wrapper.get("select").setValue("otel");
    const inputs = wrapper.findAll("input");
    await inputs[0]?.setValue("http://localhost:4318");
    await inputs[1]?.setValue("dashboard-service");
    await wrapper.get("button").trigger("click");

    expect(wrapper.emitted("update:modelValue")).toEqual([
      [expect.objectContaining({ observability_backend: "otel" })],
      [expect.objectContaining({ otel_endpoint: "http://localhost:4318" })],
      [expect.objectContaining({ otel_service_name: "dashboard-service" })],
    ]);
    expect(wrapper.emitted("save")).toHaveLength(1);
  });
});
