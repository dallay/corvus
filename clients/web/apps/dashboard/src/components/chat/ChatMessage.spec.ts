import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import ChatMessage from "@/components/chat/ChatMessage.vue";
import { i18nConfig } from "@/i18n";

const testI18n = createI18n(i18nConfig);

function mountMessage(props = {}) {
  return mount(ChatMessage, {
    props: {
      role: "assistant",
      content: "Hello!",
      ...props,
    },
    global: {
      plugins: [testI18n],
    },
  });
}

describe("ChatMessage", () => {
  it("renders assistant message content", () => {
    const wrapper = mountMessage({ role: "assistant", content: "Hi there" });
    expect(wrapper.find(".bubble-text").text()).toContain("Hi there");
  });

  it("renders user message content", () => {
    const wrapper = mountMessage({ role: "user", content: "Hello!" });
    expect(wrapper.find(".bubble-text").text()).toContain("Hello!");
  });

  it("does not show memory recall indicator when recalledMemoryKeys is absent", () => {
    const wrapper = mountMessage({ role: "assistant", content: "No memory" });
    expect(wrapper.find('[data-testid="memory-recall-indicator"]').exists()).toBe(false);
  });

  it("does not show memory recall indicator when recalledMemoryKeys is empty", () => {
    const wrapper = mountMessage({
      role: "assistant",
      content: "No memory",
      recalledMemoryKeys: [],
    });
    expect(wrapper.find('[data-testid="memory-recall-indicator"]').exists()).toBe(false);
  });

  it("shows memory recall indicator when agent recalled memory", () => {
    const wrapper = mountMessage({
      role: "assistant",
      content: "I remember you like TypeScript.",
      recalledMemoryKeys: ["memory_recall"],
    });
    expect(wrapper.find('[data-testid="memory-recall-indicator"]').exists()).toBe(true);
    expect(wrapper.find(".memory-recall-badge").exists()).toBe(true);
  });

  it("does not show memory recall indicator for user messages even if keys provided", () => {
    const wrapper = mountMessage({
      role: "user",
      content: "Hi",
      recalledMemoryKeys: ["memory_recall"],
    });
    expect(wrapper.find('[data-testid="memory-recall-indicator"]').exists()).toBe(false);
  });

  it("expands and collapses the memory recall detail on badge click", async () => {
    const wrapper = mountMessage({
      role: "assistant",
      content: "Based on memory...",
      recalledMemoryKeys: ["memory_recall"],
    });

    expect(wrapper.find(".memory-recall-detail").exists()).toBe(false);

    await wrapper.find(".memory-recall-badge").trigger("click");
    expect(wrapper.find(".memory-recall-detail").exists()).toBe(true);

    await wrapper.find(".memory-recall-badge").trigger("click");
    expect(wrapper.find(".memory-recall-detail").exists()).toBe(false);
  });

  it("sets aria-expanded on the badge correctly", async () => {
    const wrapper = mountMessage({
      role: "assistant",
      content: "Based on memory...",
      recalledMemoryKeys: ["memory_recall"],
    });

    const badge = wrapper.find(".memory-recall-badge");
    expect(badge.attributes("aria-expanded")).toBe("false");

    await badge.trigger("click");
    expect(badge.attributes("aria-expanded")).toBe("true");
  });
});
