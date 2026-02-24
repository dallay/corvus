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

describe("App", () => {
  it("agrega mensajes y limpia el prompt al enviar", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ response: "Respuesta <b>ok</b>" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );

    const wrapper = mountApp();

    // In the new design, the empty state (hero) is shown when there's only the welcome message.
    // The input is still visible at the bottom.
    const input = wrapper.get('input[placeholder="Escribe un mensaje..."]');
    await input.setValue('Hola <script>alert("x")</script>');
    await wrapper.get("form").trigger("submit.prevent");
    await flushPromises();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(String(url)).toContain("/webhook");
    expect(init?.method).toBe("POST");

    const chatMessages = wrapper.findAll('[data-testid="chat-message"]');
    const userMessage = chatMessages[chatMessages.length - 2];
    const lastMessage = chatMessages[chatMessages.length - 1];

    expect(chatMessages).toHaveLength(3);
    expect(userMessage?.html()).toContain("&lt;script&gt;alert");
    expect(userMessage?.find("script").exists()).toBe(false);
    expect(userMessage?.text()).toContain('<script>alert("x")</script>');

    expect(lastMessage?.html()).toContain("&lt;b&gt;ok&lt;/b&gt;");
    expect(lastMessage?.find("script").exists()).toBe(false);
    expect(lastMessage?.find("b").exists()).toBe(false);
    expect((input.element as HTMLInputElement).value).toBe("");
  });

  it("alterna entre configuracion y chat", async () => {
    const wrapper = mountApp();

    expect(wrapper.find('input[placeholder="Escribe un mensaje..."]').exists()).toBe(true);

    // Find the first toggle-config button (could be sidebar or mobile header)
    await wrapper.get('[data-testid="toggle-config"]').trigger("click");

    expect(wrapper.find('input[placeholder="http://127.0.0.1:3000"]').exists()).toBe(true);
    expect(wrapper.find('input[placeholder="Código de emparejamiento"]').exists()).toBe(true);
    expect(wrapper.find('input[placeholder="Token bearer"]').exists()).toBe(true);
    expect(wrapper.find('input[placeholder="Secreto del webhook"]').exists()).toBe(true);

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    expect(wrapper.find('input[placeholder="Escribe un mensaje..."]').exists()).toBe(true);
  });

  it("hace pairing y luego usa bearer token en webhook", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ paired: true, token: "zc_test_token" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ response: "ok" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

    const wrapper = mountApp();

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    await wrapper.get('input[placeholder="Código de emparejamiento"]').setValue("123456");
    await wrapper.get("form").trigger("submit.prevent");
    await flushPromises();

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    const input = wrapper.get('input[placeholder="Escribe un mensaje..."]');
    await input.setValue("hola");
    await wrapper.get("form").trigger("submit.prevent");
    await flushPromises();

    expect(fetchMock).toHaveBeenCalledTimes(2);

    const [pairUrl, pairInit] = fetchMock.mock.calls[0] ?? [];
    expect(String(pairUrl)).toContain("/pair");
    expect((pairInit?.headers as Record<string, string>)["X-Pairing-Code"]).toBe("123456");

    const [webhookUrl, webhookInit] = fetchMock.mock.calls[1] ?? [];
    expect(String(webhookUrl)).toContain("/webhook");
    expect((webhookInit?.headers as Record<string, string>).Authorization).toBe(
      "Bearer zc_test_token"
    );
  });
});
