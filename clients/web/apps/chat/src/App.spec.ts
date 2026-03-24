import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import App from "@/App.vue";
import { i18nConfig } from "@/i18n";

const testI18n = createI18n(i18nConfig);

function translatedText(key: string): string {
  return String(testI18n.global.t(key));
}

function mountApp() {
  return mount(App, {
    global: {
      plugins: [testI18n],
    },
  });
}

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
  window.sessionStorage.clear();
});

describe("App", () => {
  it("renders onboarding gating before chat becomes available", () => {
    const wrapper = mountApp();

    expect(wrapper.text()).toContain(translatedText("chatOnboarding.ready.title"));
    expect(wrapper.text()).toContain(translatedText("chatOnboarding.steps.runtime.title"));
    expect(
      wrapper.find(`input[placeholder="${translatedText("chat.inputPlaceholder")}"]`).exists()
    ).toBe(false);
  });

  it("toggles between configuration and onboarding gate", async () => {
    const wrapper = mountApp();

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");

    expect(
      wrapper.find(`input[placeholder="${translatedText("form.baseUrlPlaceholder")}"]`).exists()
    ).toBe(true);
    expect(
      wrapper.find(`input[placeholder="${translatedText("form.pairingCodePlaceholder")}"]`).exists()
    ).toBe(true);

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");

    expect(wrapper.text()).toContain(translatedText("chatOnboarding.steps.trust.title"));
  });

  it("pairs, gates on session start, and sends chat turns with bearer and session headers", async () => {
    vi.spyOn(crypto, "randomUUID")
      .mockReturnValueOnce("11111111-1111-4111-8111-111111111111")
      .mockReturnValueOnce("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa");

    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: false }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ paired: true, token: "zc_test_token" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            response: "Respuesta <b>ok</b>",
            session_id: "11111111-1111-4111-8111-111111111111",
          }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }
        )
      );

    const wrapper = mountApp();

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    await wrapper
      .get(`input[placeholder="${translatedText("form.pairingCodePlaceholder")}"]`)
      .setValue("123456");
    const pairButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("auth.pair"));
    expect(pairButton?.exists()).toBe(true);
    await pairButton!.trigger("click");
    await flushPromises();

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    const startSessionButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("chat.startSession"));
    expect(startSessionButton?.exists()).toBe(true);
    await startSessionButton!.trigger("click");
    await flushPromises();

    const input = wrapper.get(`input[placeholder="${translatedText("chat.inputPlaceholder")}"]`);
    await input.setValue('Hola <script>alert("x")</script>');
    await wrapper.get("form").trigger("submit.prevent");
    await flushPromises();

    expect(fetchMock).toHaveBeenCalledTimes(4);
    const [pairUrl, pairInit] = fetchMock.mock.calls[1] ?? [];
    expect(String(pairUrl)).toContain("/pair");
    expect((pairInit?.headers as Record<string, string>)["X-Pairing-Code"]).toBe("123456");

    const [webhookUrl, webhookInit] = fetchMock.mock.calls[3] ?? [];
    expect(String(webhookUrl)).toContain("/webhook");
    expect((webhookInit?.headers as Record<string, string>).Authorization).toBe(
      "Bearer zc_test_token"
    );
    expect((webhookInit?.headers as Record<string, string>)["X-Session-Id"]).toBe(
      "11111111-1111-4111-8111-111111111111"
    );

    const chatMessages = wrapper.findAll('[data-testid="chat-message"]');
    expect(chatMessages).toHaveLength(3);
    expect(chatMessages[1]?.html()).toContain("&lt;script&gt;alert");
    expect(chatMessages[2]?.html()).toContain("&lt;b&gt;ok&lt;/b&gt;");
  });

  it("surfaces credential recovery when a chat turn is rejected", async () => {
    vi.spyOn(crypto, "randomUUID")
      .mockReturnValueOnce("22222222-2222-4222-8222-222222222222")
      .mockReturnValueOnce("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb");

    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "Unauthorized" }), {
          status: 401,
          headers: { "Content-Type": "application/json" },
        })
      );

    const wrapper = mountApp();
    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    await wrapper
      .get(`input[placeholder="${translatedText("form.bearerTokenPlaceholder")}"]`)
      .setValue("stale-token");
    await wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("auth.connect"))
      ?.trigger("click");
    await flushPromises();

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    await wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("chat.startSession"))
      ?.trigger("click");
    await flushPromises();

    const input = wrapper.get(`input[placeholder="${translatedText("chat.inputPlaceholder")}"]`);
    await input.setValue("hola");
    await wrapper.get("form").trigger("submit.prevent");
    await flushPromises();

    expect(wrapper.text()).toContain(
      translatedText("chatOnboarding.recovery.credential_invalid.description")
    );
    expect(wrapper.text()).toContain(translatedText("chatOnboarding.steps.trust.title"));
  });
});
