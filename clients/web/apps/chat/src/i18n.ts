import { translations } from "@corvus/locales";
import { createI18n } from "vue-i18n";

export const i18nConfig = {
  legacy: false,
  locale: "es",
  messages: {
    es: translations.es,
  },
} as const;

export const i18n = createI18n(i18nConfig);
