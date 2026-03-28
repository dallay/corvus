import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import McpOverview from "@/components/config/McpOverview.vue";
import { i18nConfig } from "@/i18n";

function mountComponent() {
  return mount(McpOverview, {
    props: {
      gatewayUrl: "http://localhost:3000",
      bearerToken: "test-token",
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

describe("McpOverview", () => {
  it("renders MCP servers on successful fetch", async () => {
    const mockConfig = {
      config: {
        mcp: {
          enabled: true,
          servers: [
            {
              name: "test-server",
              enabled: true,
              command: "npx test-mcp",
              capabilities: ["tools", "prompts"],
              startup_timeout_ms: 5000,
              call_timeout_ms: 30000,
              output_limit_bytes: 1048576,
            },
          ],
        },
      },
    };

    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockConfig),
      })
    );

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find('[data-testid="mcp-server-test-server"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("test-server");
    expect(wrapper.text()).toContain("npx test-mcp");
    expect(wrapper.text()).toContain("tools, prompts");

    vi.unstubAllGlobals();
  });

  it("shows error on fetch failure", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("Network error")));

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find(".error").exists()).toBe(true);
    expect(wrapper.text()).toContain("Network error");

    vi.unstubAllGlobals();
  });
});
