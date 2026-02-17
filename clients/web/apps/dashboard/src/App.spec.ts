import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createI18n } from "vue-i18n";

import App from "@/App.vue";
import es from "@/locales/es.json";

function mountApp() {
  const i18n = createI18n({
    legacy: false,
    locale: "es",
    fallbackLocale: "es",
    messages: { es },
  });

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
    expect(chatMessages).toHaveLength(initialMessages + 2);
    expect(chatMessages.at(-1)?.text()).toContain("&lt;script&gt;alert");
    expect((input.element as HTMLInputElement).value).toBe("");
  });

  it("alterna entre configuracion y chat", async () => {
    const wrapper = mountApp();

    expect(wrapper.find('input[placeholder="Escribe un mensaje..."]').exists()).toBe(true);

    await wrapper.get("button").trigger("click");

    expect(wrapper.find('input[placeholder="http://127.0.0.1:3000"]').exists()).toBe(true);
    expect(wrapper.find('input[placeholder="Codigo de emparejamiento"]').exists()).toBe(true);
    expect(wrapper.find('input[placeholder="Token bearer"]').exists()).toBe(true);
    expect(wrapper.find('input[placeholder="Secreto del webhook"]').exists()).toBe(true);

    await wrapper.get("button").trigger("click");
    expect(wrapper.find('input[placeholder="Escribe un mensaje..."]').exists()).toBe(true);
  });
});
