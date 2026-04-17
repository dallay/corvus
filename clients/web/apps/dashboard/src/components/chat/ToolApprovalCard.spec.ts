import { mount } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import ToolApprovalCard from "@/components/chat/ToolApprovalCard.vue";
import { i18nConfig } from "@/i18n";

const testI18n = createI18n(i18nConfig);
const mountedWrappers = new Set<ReturnType<typeof mount>>();

function mountCard(props = {}) {
  const wrapper = mount(ToolApprovalCard, {
    attachTo: document.body,
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

  mountedWrappers.add(wrapper);
  return wrapper;
}

afterEach(() => {
  for (const wrapper of mountedWrappers) {
    wrapper.unmount();
  }
  mountedWrappers.clear();
  document.body.innerHTML = "";
});

describe("ToolApprovalCard", () => {
  it("renders tool name and reason", () => {
    const wrapper = mountCard();

    expect(wrapper.get('[data-testid="tool-name"]').text()).toBe("file_write");
    expect(wrapper.get('[data-testid="tool-reason"]').text()).toBe(
      "This tool modifies the filesystem"
    );
    expect(wrapper.get('[data-testid="tool-approval"]').attributes("role")).toBe("group");
  });

  it("focuses the primary action on mount and wires descriptions", () => {
    const wrapper = mountCard();

    expect(document.activeElement).toBe(wrapper.get('[data-testid="btn-approve"]').element);
    expect(wrapper.get('[data-testid="tool-approval"]').attributes("aria-labelledby")).toBe(
      "tool-approval-title-approval-42"
    );
    expect(wrapper.get('[data-testid="tool-approval"]').attributes("aria-describedby")).toBe(
      "tool-approval-reason-approval-42"
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
