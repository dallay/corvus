import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
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

describe("App", () => {
  it("agrega mensajes y limpia el prompt al enviar", async () => {
    const wrapper = mountApp();

    const initialMessages = wrapper.findAll('[data-testid="chat-message"]').length;
    const input = wrapper.get('input[placeholder="Escribe un mensaje..."]');
    await input.setValue('Hola <script>alert("x")</script>');
    await wrapper.get("form").trigger("submit.prevent");

    const chatMessages = wrapper.findAll('[data-testid="chat-message"]');
    const lastMessage = chatMessages[chatMessages.length - 1];
    expect(chatMessages).toHaveLength(initialMessages + 2);
    // Contract: ChatMessage renders with text interpolation (never v-html).
    expect(lastMessage?.html()).toContain("&lt;script&gt;alert");
    expect(lastMessage?.find("script").exists()).toBe(false);
    expect(lastMessage?.text()).toContain('<script>alert("x")</script>');
    expect((input.element as HTMLInputElement).value).toBe("");
  });

  it("alterna entre configuracion y chat", async () => {
    const wrapper = mountApp();

    expect(wrapper.find('input[placeholder="Escribe un mensaje..."]').exists()).toBe(true);

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");

    expect(wrapper.find('input[placeholder="http://127.0.0.1:3000"]').exists()).toBe(true);
    expect(wrapper.find('input[placeholder="Código de emparejamiento"]').exists()).toBe(true);
    expect(wrapper.find('input[placeholder="Token bearer"]').exists()).toBe(true);
    expect(wrapper.find('input[placeholder="Secreto del webhook"]').exists()).toBe(true);

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    expect(wrapper.find('input[placeholder="Escribe un mensaje..."]').exists()).toBe(true);
  });
});
