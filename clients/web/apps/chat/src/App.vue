<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";

// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import ChatMessage from "@/components/chat/ChatMessage.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import Button from "@/components/ui/button/Button.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import Input from "@/components/ui/input/Input.vue";

type Role = "assistant" | "user";

interface Message {
  id: number;
  role: Role;
  content: string;
}

type SecretField = "pairingCode" | "bearerToken" | "webhookSecret";

const ALLOWED_LOCAL_HOSTS = new Set(["localhost", "127.0.0.1", "[::1]"]);

const MAX_PROMPT_LENGTH = 500;
const modelName = "Corvus Agent";
const { t } = useI18n();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const showConfig = ref(false);
const prompt = ref("");
const baseUrl = ref("http://127.0.0.1:3000");
const chatContainer = ref<HTMLDivElement | null>(null);
const secretInputNonce = ref(0);
const saveStatus = ref<"idle" | "saving" | "success" | "error">("idle");
const saveErrorMessage = ref("");

let saveStatusTimeoutId: ReturnType<typeof setTimeout> | null = null;
let requestTimeoutId: ReturnType<typeof setTimeout> | null = null;

let messageIdCounter = 1;
let pairingCodeInput = "";
let bearerTokenInput = "";
let webhookSecretInput = "";

function createIdempotencyKey(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }

  return `chat-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

const messages = ref<Message[]>([
  {
    id: 0,
    role: "assistant",
    content: t("chat.welcome", { modelName }),
  },
]);

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const canSend = computed(() => prompt.value.trim().length > 0);

function nextMessageId(): number {
  const currentId = messageIdCounter;
  messageIdCounter += 1;
  return currentId;
}

function resetSaveStatus(): void {
  if (saveStatus.value === "saving") {
    return;
  }

  if (saveStatusTimeoutId) {
    clearTimeout(saveStatusTimeoutId);
    saveStatusTimeoutId = null;
  }
  saveStatus.value = "idle";
}

/**
 * Returns true when the URL is safe to send secrets to:
 * - HTTPS is always allowed
 * - HTTP is only allowed for local hosts (localhost, 127.0.0.1, [::1])
 */
function isUrlSafeForSecrets(rawUrl: string): boolean {
  let parsed: URL;
  try {
    parsed = new URL(rawUrl);
  } catch {
    return false;
  }
  if (parsed.protocol === "https:") {
    return true;
  }
  return parsed.protocol === "http:" && ALLOWED_LOCAL_HOSTS.has(parsed.hostname);
}

function handlePairingError(response: Response): Error {
  if (response.status === 403) {
    return new Error(t("form.pairingInvalidError"));
  }
  if (response.status === 429) {
    return new Error(t("form.pairingRateLimitError"));
  }
  return new Error(`HTTP ${response.status}`);
}

async function executePairing(pairingCode: string, gatewayBaseUrl: string): Promise<string> {
  const controller = new AbortController();
  requestTimeoutId = setTimeout(() => controller.abort(), 10000);

  try {
    const pairEndpoint = new URL("/pair", gatewayBaseUrl);
    const response = await fetch(pairEndpoint.toString(), {
      method: "POST",
      headers: {
        "X-Pairing-Code": pairingCode.trim(),
      },
      signal: controller.signal,
    });

    if (!response.ok) {
      throw handlePairingError(response);
    }

    const pairResult = (await response.json()) as {
      token?: string;
      paired?: boolean;
    };
    if (!pairResult.paired || !pairResult.token) {
      throw new Error(t("form.pairingMissingTokenError"));
    }

    return pairResult.token;
  } finally {
    if (requestTimeoutId) {
      clearTimeout(requestTimeoutId);
      requestTimeoutId = null;
    }
  }
}

function handleSaveError(error: unknown): string {
  if (error instanceof Error && error.name === "AbortError") {
    return t("form.timeoutError") || "Request timeout";
  }
  if (error instanceof Error && error.message) {
    return error.message;
  }
  return t("form.saveError");
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function captureSecretInput(field: SecretField, value: string): void {
  resetSaveStatus();
  if (field === "pairingCode") {
    pairingCodeInput = value;
    return;
  }
  if (field === "bearerToken") {
    bearerTokenInput = value;
    return;
  }
  webhookSecretInput = value;
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function saveGatewayConfig(): Promise<void> {
  resetSaveStatus();
  saveStatus.value = "saving";
  saveErrorMessage.value = "";

  const gatewayBaseUrl = baseUrl.value.replace(/\/$/, "");
  const hasSecrets = !!(pairingCodeInput || bearerTokenInput || webhookSecretInput);

  if (hasSecrets && !isUrlSafeForSecrets(gatewayBaseUrl)) {
    saveStatus.value = "error";
    saveErrorMessage.value = t("form.insecureUrlError");
    return;
  }

  if (!pairingCodeInput.trim()) {
    showSaveSuccess();
    return;
  }

  try {
    const token = await executePairing(pairingCodeInput, gatewayBaseUrl);
    bearerTokenInput = token;
    pairingCodeInput = "";
    webhookSecretInput = "";
    secretInputNonce.value += 1;
    showSaveSuccess();
  } catch (error) {
    saveStatus.value = "error";
    saveErrorMessage.value = handleSaveError(error);
    console.error("Error saving gateway config", error);
  }
}

function showSaveSuccess(): void {
  saveStatus.value = "success";
  saveStatusTimeoutId = setTimeout(() => {
    saveStatus.value = "idle";
  }, 3000);
}

function scrollChatToBottom(): void {
  if (!chatContainer.value) {
    return;
  }
  chatContainer.value.scrollTop = chatContainer.value.scrollHeight;
}

function buildRequestHeaders(): Record<string, string> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    "X-Idempotency-Key": createIdempotencyKey(),
  };

  if (bearerTokenInput.trim()) {
    headers.Authorization = `Bearer ${bearerTokenInput.trim()}`;
  }
  if (webhookSecretInput.trim()) {
    headers["X-Webhook-Secret"] = webhookSecretInput.trim();
  }

  return headers;
}

function handleChatError(error: unknown, normalizedText: string): string {
  if (error instanceof Error && error.name === "AbortError") {
    return t("chat.timeoutError");
  }
  if (error instanceof Error && error.message) {
    return error.message;
  }
  return t("chat.requestError", {
    text: normalizedText,
    modelName,
  });
}

function handleChatResponseError(response: Response): Error {
  if (response.status === 401) {
    return new Error(t("chat.unauthorizedError"));
  }
  if (response.status === 429) {
    return new Error(t("chat.rateLimitError"));
  }
  return new Error(`HTTP ${response.status}`);
}

function updateAssistantMessage(messageId: number, content: string): void {
  const messageIndex = messages.value.findIndex((item) => item.id === messageId);
  if (messageIndex >= 0) {
    messages.value[messageIndex] = {
      id: messageId,
      role: "assistant",
      content,
    };
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function sendMessage(): Promise<void> {
  const text = prompt.value.trim();
  if (!text) {
    return;
  }

  const normalizedText = text.slice(0, MAX_PROMPT_LENGTH);
  const gatewayBaseUrl = baseUrl.value.replace(/\/$/, "");

  messages.value.push({
    id: nextMessageId(),
    role: "user",
    content: normalizedText,
  });

  const assistantMessageId = nextMessageId();
  messages.value.push({
    id: assistantMessageId,
    role: "assistant",
    content: t("chat.processing", {
      text: normalizedText,
      modelName,
      gateway: gatewayBaseUrl,
    }),
  });

  prompt.value = "";
  await nextTick();
  scrollChatToBottom();

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), 15000);

  try {
    const endpoint = new URL("/webhook", gatewayBaseUrl);
    const response = await fetch(endpoint.toString(), {
      method: "POST",
      headers: buildRequestHeaders(),
      body: JSON.stringify({
        message: normalizedText,
      }),
      signal: controller.signal,
    });

    if (!response.ok) {
      throw handleChatResponseError(response);
    }

    const data = (await response.json()) as {
      response?: string;
      message?: string;
      text?: string;
    };
    const assistantText = data.response ?? data.message ?? data.text ?? t("chat.emptyResponse");

    updateAssistantMessage(assistantMessageId, assistantText);
  } catch (error) {
    console.error("Error sending chat message", error);
    const errorMessage = handleChatError(error, normalizedText);
    updateAssistantMessage(assistantMessageId, errorMessage);
  } finally {
    clearTimeout(timeoutId);
  }

  await nextTick();
  scrollChatToBottom();
}

onUnmounted(() => {
  if (saveStatusTimeoutId) clearTimeout(saveStatusTimeoutId);
  if (requestTimeoutId) clearTimeout(requestTimeoutId);
});
</script>

<template>
  <div class="app-shell">
    <!-- ── Sidebar ──────────────────────────────────────── -->
    <aside class="sidebar">
      <!-- Sidebar Header -->
      <div class="sidebar-header">
        <div class="logo-icon animate-pulse-glow">
          <img src="/favicon-light.svg" alt="Corvus" width="20" height="20" />
        </div>
        <div>
          <h1 class="sidebar-title">Corvus AI</h1>
          <p class="sidebar-subtitle">{{ t("app.simpleChat") }}</p>
        </div>
      </div>

      <!-- Nav Items -->
      <nav class="sidebar-nav">
        <button class="nav-item nav-item--active">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
          </svg>
          {{ t("chat.newChat") }}
        </button>
      </nav>

      <!-- Sidebar Footer -->
      <div class="sidebar-footer">
        <button
          data-testid="toggle-config"
          class="nav-item"
          @click="showConfig = !showConfig"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09A1.65 1.65 0 0 0 15 4.6a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9c.23.5.8.83 1.4.83H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09c-.6 0-1.17.33-1.51.83Z" />
          </svg>
          {{ showConfig ? t("app.backToChat") : t("app.config") }}
        </button>
      </div>
    </aside>

    <!-- ── Main Content ─────────────────────────────────── -->
    <main class="main-content">
      <!-- Mobile Header -->
      <header class="mobile-header">
        <div class="mobile-header-left">
          <div class="logo-icon logo-icon--sm">
            <img src="/favicon-light.svg" alt="Corvus" width="16" height="16" />
          </div>
          <span class="mobile-title">Corvus AI</span>
        </div>
        <button
          data-testid="toggle-config"
          class="icon-btn"
          @click="showConfig = !showConfig"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09A1.65 1.65 0 0 0 15 4.6a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9c.23.5.8.83 1.4.83H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09c-.6 0-1.17.33-1.51.83Z" />
          </svg>
        </button>
      </header>

      <!-- ── Config Panel ───────────────────────────── -->
      <div v-if="showConfig" class="config-wrapper">
        <form class="config-card animate-slide-up" @submit.prevent="saveGatewayConfig">
          <div class="config-header">
            <div class="config-header-icon">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09A1.65 1.65 0 0 0 15 4.6a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9c.23.5.8.83 1.4.83H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09c-.6 0-1.17.33-1.51.83Z" />
              </svg>
            </div>
            <div>
              <h2 class="config-title">{{ t("app.gatewayConfig") }}</h2>
              <p class="config-subtitle">{{ t("form.configSubtitle") }}</p>
            </div>
          </div>

          <label class="field">
            <span class="field-label">{{ t("form.baseUrl") }}</span>
            <Input
              v-model="baseUrl"
              :placeholder="t('form.baseUrlPlaceholder')"
              @update:model-value="resetSaveStatus"
            />
          </label>

          <label class="field">
            <span class="field-label">{{ t("form.pairingCode") }}</span>
            <Input
              :key="`pairing-${secretInputNonce}`"
              :placeholder="t('form.pairingCodePlaceholder')"
              type="password"
              @update:model-value="(value: string) => captureSecretInput('pairingCode', value)"
            />
          </label>

          <label class="field">
            <span class="field-label">{{ t("form.bearerToken") }}</span>
            <Input
              :key="`bearer-${secretInputNonce}`"
              :placeholder="t('form.bearerTokenPlaceholder')"
              type="password"
              @update:model-value="(value: string) => captureSecretInput('bearerToken', value)"
            />
          </label>

          <label class="field">
            <span class="field-label">{{ t("form.webhookSecret") }}</span>
            <Input
              :key="`webhook-${secretInputNonce}`"
              :placeholder="t('form.webhookSecretPlaceholder')"
              type="password"
              @update:model-value="(value: string) => captureSecretInput('webhookSecret', value)"
            />
          </label>

          <div class="config-actions">
            <Button :disabled="saveStatus === 'saving'" type="submit" class="w-full">
              {{ t("form.save") }}
            </Button>
          </div>

          <div v-if="saveStatus === 'success'" class="alert alert--success animate-fade-in">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
              <path d="M20 6L9 17l-5-5" />
            </svg>
            {{ t("form.saveSuccess") }}
          </div>

          <div v-if="saveStatus === 'error'" class="alert alert--error animate-fade-in">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
              <circle cx="12" cy="12" r="10" />
              <line x1="15" y1="9" x2="9" y2="15" />
              <line x1="9" y1="9" x2="15" y2="15" />
            </svg>
            {{ saveErrorMessage }}
          </div>
        </form>
      </div>

      <!-- ── Chat Area ──────────────────────────────── -->
      <template v-else>
        <!-- Empty state (only welcome) -->
        <div v-if="messages.length <= 1" class="hero-state">
          <div class="hero-content animate-slide-up">
            <div class="hero-icon animate-pulse-glow">
              <img src="/favicon-light.svg" alt="Corvus" width="32" height="32" />
            </div>
            <h2 class="hero-title">{{ modelName }}</h2>
            <p class="hero-subtitle">{{ t("chat.welcome", { modelName }) }}</p>
          </div>
        </div>

        <!-- Messages -->
        <div v-else ref="chatContainer" class="chat-messages">
          <div class="chat-messages-inner">
            <ChatMessage
              v-for="message in messages"
              :key="message.id"
              :role="message.role"
              :content="message.content"
            />
          </div>
        </div>

        <!-- ── Input Bar ──────────────────────────── -->
        <div class="input-bar">
          <form class="input-bar-inner" @submit.prevent="sendMessage">
            <div class="input-wrapper">
              <input
                id="chat-prompt-input"
                v-model="prompt"
                :aria-label="t('chat.inputPlaceholder')"
                :placeholder="t('chat.inputPlaceholder')"
                class="chat-input"
              />
            </div>
            <button type="submit" :disabled="!canSend" class="send-btn" :aria-label="t('chat.send')">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                <line x1="22" y1="2" x2="11" y2="13" />
                <polygon points="22 2 15 22 11 13 2 9 22 2" />
              </svg>
            </button>
          </form>
          <p class="input-disclaimer">{{ t("chat.disclaimer") }}</p>
        </div>
      </template>
    </main>
  </div>
</template>

<style scoped>
/* ── Layout ─────────────────────────────────────────────────── */

.app-shell {
  display: flex;
  height: 100vh;
  width: 100vw;
  background: var(--color-bg-primary);
  overflow: hidden;
}

/* ── Sidebar ────────────────────────────────────────────────── */

.sidebar {
  display: none;
  width: 260px;
  flex-direction: column;
  border-right: 1px solid var(--color-border);
  background: var(--color-bg-secondary);
}

@media (min-width: 768px) {
  .sidebar {
    display: flex;
  }
}

.sidebar-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 20px;
  border-bottom: 1px solid var(--color-border);
}

.sidebar-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.sidebar-subtitle {
  font-size: 12px;
  color: var(--color-text-muted);
  margin: 0;
}

.sidebar-nav {
  flex: 1;
  padding: 12px;
}

.sidebar-footer {
  padding: 12px;
  border-top: 1px solid var(--color-border);
}

/* ── Logo Icon ──────────────────────────────────────────────── */

.logo-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 12px;
  background: var(--color-accent-subtle);
  color: var(--color-accent);
  flex-shrink: 0;
}

.logo-icon--sm {
  width: 32px;
  height: 32px;
  border-radius: 8px;
}

/* ── Nav Item ───────────────────────────────────────────────── */

.nav-item {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 12px;
  border-radius: 12px;
  padding: 10px 12px;
  font-size: 14px;
  color: var(--color-text-secondary);
  background: transparent;
  border: 1px solid transparent;
  cursor: pointer;
  transition: all 0.2s;
  font-family: inherit;
}

.nav-item:hover {
  color: var(--color-text-primary);
  background: var(--color-surface-glass-hover);
}

.nav-item--active {
  color: var(--color-text-primary);
  background: var(--color-surface-glass);
  border-color: var(--color-border-accent);
}

.nav-item--active svg {
  color: var(--color-accent);
}

/* ── Icon Button ────────────────────────────────────────────── */

.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: transparent;
  border: none;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.2s;
}

.icon-btn:hover {
  color: var(--color-text-primary);
  background: var(--color-surface-glass-hover);
}

/* ── Main Content ───────────────────────────────────────────── */

.main-content {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}

/* ── Mobile Header ──────────────────────────────────────────── */

.mobile-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-secondary);
}

@media (min-width: 768px) {
  .mobile-header {
    display: none;
  }
}

.mobile-header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.mobile-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-primary);
}

/* ── Config Panel ───────────────────────────────────────────── */

.config-wrapper {
  display: flex;
  flex: 1;
  align-items: flex-start;
  justify-content: center;
  padding: 24px;
  overflow-y: auto;
}

.config-card {
  width: 100%;
  max-width: 480px;
  display: flex;
  flex-direction: column;
  gap: 20px;
  border-radius: 16px;
  border: 1px solid var(--color-border);
  background: var(--color-bg-secondary);
  padding: 24px;
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.4);
}

.config-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 4px;
}

.config-header-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 12px;
  background: var(--color-accent-subtle);
  color: var(--color-accent);
  flex-shrink: 0;
}

.config-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.config-subtitle {
  font-size: 12px;
  color: var(--color-text-muted);
  margin: 0;
}

.config-actions {
  padding-top: 8px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-secondary);
}

/* ── Alerts ─────────────────────────────────────────────────── */

.alert {
  display: flex;
  align-items: center;
  gap: 8px;
  border-radius: 12px;
  padding: 12px 16px;
  font-size: 14px;
}

.alert--success {
  background: var(--color-accent-subtle);
  color: var(--color-accent);
}

.alert--error {
  background: rgba(239, 68, 68, 0.1);
  color: var(--color-error);
}

/* ── Hero State ─────────────────────────────────────────────── */

.hero-state {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 32px;
}

.hero-content {
  text-align: center;
  max-width: 400px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.hero-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 64px;
  height: 64px;
  border-radius: 16px;
  background: var(--color-accent-subtle);
  color: var(--color-accent);
}

.hero-title {
  font-size: 24px;
  font-weight: 600;
  background: linear-gradient(to right, var(--color-accent), #6ee7b7);
  background-clip: text;
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  margin: 0;
}

.hero-subtitle {
  font-size: 14px;
  color: var(--color-text-muted);
  line-height: 1.6;
  margin: 0;
}

/* ── Chat Messages ──────────────────────────────────────────── */

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 24px 16px;
}

@media (min-width: 768px) {
  .chat-messages {
    padding: 24px 32px;
  }
}

@media (min-width: 1024px) {
  .chat-messages {
    padding: 24px 64px;
  }
}

.chat-messages-inner {
  max-width: 768px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

/* ── Input Bar ──────────────────────────────────────────────── */

.input-bar {
  border-top: 1px solid var(--color-border);
  background: var(--color-bg-secondary);
  padding: 16px;
}

.input-bar-inner {
  max-width: 768px;
  margin: 0 auto;
  display: flex;
  align-items: center;
  gap: 12px;
}

.input-wrapper {
  flex: 1;
  display: flex;
  align-items: center;
  border-radius: 16px;
  border: 1px solid var(--color-border);
  background: var(--color-bg-input);
  padding: 0 16px;
  transition: all 0.2s;
}

.input-wrapper:focus-within {
  border-color: var(--color-border-accent);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.1);
}

.chat-input {
  flex: 1;
  background: transparent;
  border: none;
  padding: 12px 0;
  font-size: 14px;
  font-family: inherit;
  color: var(--color-text-primary);
  outline: none;
}

.chat-input::placeholder {
  color: var(--color-text-muted);
}

.send-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: 16px;
  border: none;
  background: var(--color-accent);
  color: white;
  cursor: pointer;
  transition: all 0.2s;
  flex-shrink: 0;
  box-shadow: 0 4px 12px var(--color-accent-glow);
}

.send-btn:hover:not(:disabled) {
  background: var(--color-accent-hover);
  box-shadow: 0 6px 20px var(--color-accent-glow);
  transform: translateY(-1px);
}

.send-btn:active:not(:disabled) {
  transform: scale(0.95);
}

.send-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
  box-shadow: none;
}

.input-disclaimer {
  margin: 8px 0 0;
  text-align: center;
  font-size: 12px;
  color: var(--color-text-muted);
}
</style>
