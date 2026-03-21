import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import RuntimeSettings from "@/components/config/RuntimeSettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("RuntimeSettings", () => {
  it("emits runtime kind changes and disables save while saving", async () => {
    const wrapper = mount(RuntimeSettings, {
      props: {
        modelValue: createAdminConfigForm({
          runtime_kind: "native",
        }),
        runtimeKindOptions: ["native", "docker"],
        disabled: false,
        saving: true,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("Runtime kind");
    expect(wrapper.get("button").attributes("disabled")).toBeDefined();

    await wrapper.get("select").setValue("docker");

    expect(wrapper.emitted("update:modelValue")).toEqual([
      [expect.objectContaining({ runtime_kind: "docker" })],
    ]);
  });
});
