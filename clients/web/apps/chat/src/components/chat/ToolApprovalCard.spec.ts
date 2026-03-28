import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import ToolApprovalCard from "@/components/chat/ToolApprovalCard.vue";
import { i18nConfig } from "@/i18n";

const testI18n = createI18n(i18nConfig);

function mountCard(props = {}) {
  return mount(ToolApprovalCard, {
    props: {
      toolName: "file_write",
      reason: "This tool modifies the filesystem",
      approvalId: "approval-42",
      ...props,
    },
    global: {
      plugins: [testI18n],
    },
  });
}

describe("ToolApprovalCard", () => {
  it("renders tool name and reason", () => {
    const wrapper = mountCard();

    expect(wrapper.get('[data-testid="tool-name"]').text()).toBe("file_write");
    expect(wrapper.get('[data-testid="tool-reason"]').text()).toBe(
      "This tool modifies the filesystem"
    );
  });

  it("emits approve with approval ID", async () => {
    const wrapper = mountCard();

    await wrapper.get('[data-testid="btn-approve"]').trigger("click");

    expect(wrapper.emitted("approve")).toEqual([["approval-42"]]);
  });

  it("emits reject with approval ID", async () => {
    const wrapper = mountCard();

    await wrapper.get('[data-testid="btn-reject"]').trigger("click");

    expect(wrapper.emitted("reject")).toEqual([["approval-42"]]);
  });
});
