import { createI18n } from "vue-i18n";

import { translations } from "@corvus/locales";

export const i18nConfig = {
  legacy: false,
  locale: "es",
  messages: {
    es: translations.es,
  },
} as const;

export const i18n = createI18n(i18nConfig);
