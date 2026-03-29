import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import SessionFilters from "@/components/sessions/SessionFilters.vue";
import { i18nConfig } from "@/i18n";

function mountFilters() {
  return mount(SessionFilters, {
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("SessionFilters", () => {
  it("clamps an invalid status selection back to all", async () => {
    const wrapper = mountFilters();
    const statusSelect = wrapper.findAll("select")[0];
    expect(statusSelect).toBeDefined();
    if (!statusSelect) {
      throw new Error("status select not found");
    }

    await statusSelect.setValue("active");
    (statusSelect.element as HTMLSelectElement).value = "unexpected-status";
    await statusSelect.trigger("change");

    const emitted = wrapper.emitted("update:status");
    expect(emitted?.at(-1)).toEqual([undefined]);
    expect((statusSelect.element as HTMLSelectElement).value).toBe("all");
  });

  it("clamps an invalid sort selection back to last_activity", async () => {
    const wrapper = mountFilters();
    const sortSelect = wrapper.findAll("select")[1];
    expect(sortSelect).toBeDefined();
    if (!sortSelect) {
      throw new Error("sort select not found");
    }

    await sortSelect.setValue("started_at");
    (sortSelect.element as HTMLSelectElement).value = "unexpected-sort";
    await sortSelect.trigger("change");

    const emitted = wrapper.emitted("update:sort");
    expect(emitted?.at(-1)).toEqual(["last_activity"]);
    expect((sortSelect.element as HTMLSelectElement).value).toBe("last_activity");
  });
});
