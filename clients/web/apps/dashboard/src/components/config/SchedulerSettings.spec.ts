import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import SchedulerSettings from "@/components/config/SchedulerSettings.vue";
import { i18nConfig } from "@/i18n";
import { createAdminConfigForm } from "@/test/adminConfigFormFactory";

describe("SchedulerSettings", () => {
  it("emits checkbox and numeric field updates and allows saving", async () => {
    const wrapper = mount(SchedulerSettings, {
      props: {
        modelValue: createAdminConfigForm({
          scheduler_enabled: true,
          scheduler_max_tasks: "64",
          scheduler_max_concurrent: "4",
        }),
        disabled: false,
        saving: false,
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });

    expect(wrapper.text()).toContain("Scheduler enabled");

    const checkbox = wrapper.get('input[type="checkbox"]');
    await checkbox.setValue(false);

    const numberInputs = wrapper.findAll('input[type="number"]');
    await numberInputs[0]?.setValue("96");
    await numberInputs[1]?.setValue("8");
    await wrapper.get("button").trigger("click");

    expect(wrapper.emitted("update:modelValue")).toEqual([
      [expect.objectContaining({ scheduler_enabled: false })],
      [expect.objectContaining({ scheduler_max_tasks: "96" })],
      [expect.objectContaining({ scheduler_max_concurrent: "8" })],
    ]);
    expect(wrapper.emitted("save")).toHaveLength(1);
  });
});
