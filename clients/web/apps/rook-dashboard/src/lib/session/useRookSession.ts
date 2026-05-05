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
  return safeGetSessionItem(key);
}

function writeValue(key: string, value: string): void {
  const trimmed = value.trim();
  if (!trimmed) {
    safeRemoveSessionItem(key);
    return;
  }

  safeSetSessionItem(key, trimmed);
}

function safeGetSessionItem(key: string): string {
  try {
    return globalThis.window?.sessionStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

function safeSetSessionItem(key: string, value: string): void {
  try {
    globalThis.window?.sessionStorage.setItem(key, value);
  } catch {
    // Storage can be unavailable in private, SSR, or restricted contexts.
  }
}

function safeRemoveSessionItem(key: string): void {
  try {
    globalThis.window?.sessionStorage.removeItem(key);
  } catch {
    // Storage can be unavailable in private, SSR, or restricted contexts.
  }
}
