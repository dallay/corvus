import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import LocalMemoryCategoryChart from "@/components/memory/LocalMemoryCategoryChart.vue";

const facets = [
  { category: "Core", total: 4, sessionCount: 2, isActive: true },
  { category: "Conversation", total: 2, sessionCount: 1, isActive: false },
];

describe("LocalMemoryCategoryChart", () => {
  it("renders category totals and emits category selection", async () => {
    const wrapper = mount(LocalMemoryCategoryChart, {
      props: { facets },
    });

    await wrapper.findAll("button.category-bar")[1]?.trigger("click");

    expect(wrapper.text()).toContain("Core");
    expect(wrapper.text()).toContain("Conversation");
    expect(wrapper.emitted("select-category")).toEqual([["Conversation"]]);
  });

  it("offers a clear-focus action when a category is active", async () => {
    const wrapper = mount(LocalMemoryCategoryChart, {
      props: { facets },
    });

    await wrapper.find("button.clear-category-focus").trigger("click");

    expect(wrapper.emitted("clear-category")).toEqual([[]]);
  });

  it("pluralizes the session label correctly", () => {
    const wrapper = mount(LocalMemoryCategoryChart, {
      props: {
        facets: [{ category: "Conversation", total: 1, sessionCount: 1, isActive: false }],
      },
    });

    expect(wrapper.text()).toContain("1 session");
    expect(wrapper.text()).not.toContain("1 sessions");
  });
});
