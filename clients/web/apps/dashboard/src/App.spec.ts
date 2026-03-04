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

describe("Dashboard App", () => {
  it("renders modular config sections", () => {
    const wrapper = mountApp();
    expect(wrapper.text()).toContain("Configuración base");
    expect(wrapper.text()).toContain("Autonomía");
    expect(wrapper.text()).toContain("Gateway");
  });
});
