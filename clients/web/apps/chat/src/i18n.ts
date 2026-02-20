import { createI18n } from "vue-i18n";

import es from "@/locales/es.json";

export const i18nConfig = {
  legacy: false,
  locale: "es",
  messages: {
    es,
  },
} as const;

export const i18n = createI18n(i18nConfig);
