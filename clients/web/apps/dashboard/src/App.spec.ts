import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import App from "@/App.vue";
import { i18nConfig } from "@/i18n";

function mountApp() {
  const i18n = createI18n(i18nConfig);

  return mount(App, {
    global: {
      plugins: [i18n],
    },
  });
}

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

describe("Dashboard App", () => {
  it("hace pairing y guarda config admin", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ token: "zc_test_token" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ memory_backends: ["sqlite", "none"] }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            config: {
              default_provider: "openrouter",
              default_model: "anthropic/claude-sonnet-4",
              default_temperature: 0.7,
              memory_backend: "sqlite",
              gateway: {
                port: 3000,
                host: "127.0.0.1",
                require_pairing: true,
                allow_public_bind: false,
              },
              channels: {
                webhook_port: 3000,
                webhook_has_secret: false,
              },
            },
          }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }
        )
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ updated: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

    const wrapper = mountApp();

    const pairingInput = wrapper.get('input[type="password"]');
    await pairingInput.setValue("123456");
    const buttons = wrapper.findAll("button");

    await buttons[0]?.trigger("click");
    await flushPromises();
    await buttons[1]?.trigger("click");
    await flushPromises();
    await buttons[2]?.trigger("click");
    await flushPromises();

    expect(fetchMock).toHaveBeenCalledTimes(4);

    const [pairUrl, pairInit] = fetchMock.mock.calls[0] ?? [];
    expect(String(pairUrl)).toContain("/pair");
    expect((pairInit?.headers as Record<string, string>)["X-Pairing-Code"]).toBe("123456");

    const [saveUrl, saveInit] = fetchMock.mock.calls[3] ?? [];
    expect(String(saveUrl)).toContain("/web/admin/config");
    expect(saveInit?.method).toBe("PUT");
    expect((saveInit?.headers as Record<string, string>).Authorization).toBe(
      "Bearer zc_test_token"
    );
  });
});
