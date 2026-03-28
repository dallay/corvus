import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";
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
  afterEach(() => {
    vi.unstubAllGlobals();
  });

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

    const fetchSpy = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockConfig),
    });
    vi.stubGlobal("fetch", fetchSpy);

    const wrapper = mountComponent();
    await flushPromises();

    expect(fetchSpy).toHaveBeenCalledWith(
      expect.stringContaining("/web/admin/config"),
      expect.objectContaining({
        headers: { Authorization: "Bearer test-token" },
      })
    );

    expect(wrapper.find('[data-testid="mcp-server-test-server"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("test-server");
    expect(wrapper.text()).toContain("npx test-mcp");
    expect(wrapper.text()).toContain("tools, prompts");
  });

  it("shows error on fetch failure", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("Network error")));

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find(".error").exists()).toBe(true);
    expect(wrapper.text()).toContain("Network error");
  });

  it("refetches when gateway props change", async () => {
    const fetchSpy = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ config: { mcp: { enabled: false, servers: [] } } }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );
    vi.stubGlobal("fetch", fetchSpy);

    const wrapper = mountComponent();
    await flushPromises();
    await wrapper.setProps({ gatewayUrl: "https://gateway.example.test", bearerToken: "next" });
    await flushPromises();

    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy).toHaveBeenLastCalledWith(
      "https://gateway.example.test/web/admin/config",
      expect.objectContaining({
        headers: { Authorization: "Bearer next" },
      })
    );
  });

  it("skips the request when gatewayUrl is invalid", async () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal("fetch", fetchSpy);

    const wrapper = mount(McpOverview, {
      props: {
        gatewayUrl: "",
        bearerToken: "test-token",
      },
      global: {
        plugins: [createI18n({ ...i18nConfig, locale: "en" })],
      },
    });
    await flushPromises();

    expect(fetchSpy).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain("Invalid gateway URL");
  });
});
