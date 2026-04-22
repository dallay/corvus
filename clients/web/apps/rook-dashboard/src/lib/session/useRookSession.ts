import { computed, ref, watch } from "vue";

const BASE_URL_KEY = "rook-dashboard.base-url";
const TOKEN_KEY = "rook-dashboard.bearer-token";

export function useRookSession() {
  const baseUrl = ref(readValue(BASE_URL_KEY));
  const bearerToken = ref(readValue(TOKEN_KEY));

  watch(baseUrl, (value) => writeValue(BASE_URL_KEY, value));
  watch(bearerToken, (value) => writeValue(TOKEN_KEY, value));

  const isConfigured = computed(
    () => baseUrl.value.trim().length > 0 && bearerToken.value.trim().length > 0
  );

  return {
    baseUrl,
    bearerToken,
    isConfigured,
  };
}

function readValue(key: string): string {
  if (typeof window === "undefined") {
    return "";
  }

  return window.sessionStorage.getItem(key) ?? "";
}

function writeValue(key: string, value: string): void {
  if (typeof window === "undefined") {
    return;
  }

  const trimmed = value.trim();
  if (!trimmed) {
    window.sessionStorage.removeItem(key);
    return;
  }

  window.sessionStorage.setItem(key, trimmed);
}
