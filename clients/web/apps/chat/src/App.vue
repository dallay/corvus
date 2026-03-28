<script setup lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import { Button } from "@corvus/ui";
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import ConfigPanel from "@/components/ConfigPanel.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import ChatMessage from "@/components/chat/ChatMessage.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import ToolApprovalCard from "@/components/chat/ToolApprovalCard.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import HealthIndicator from "@/components/HealthIndicator.vue";
import { useChat } from "@/composables/useChat";
import { useGateway } from "@/composables/useGateway";

type Role = "assistant" | "user";
type MessageStatus = "streaming" | "complete" | "error";

interface Message {
  id: number;
  role: Role;
  content: string;
  status?: MessageStatus;
  approvalId?: string;
  toolName?: string;
  reason?: string;
}

const MAX_PROMPT_LENGTH = 500;
const modelName = "Corvus Agent";
const { t } = useI18n();

const showConfig = ref(false);
const prompt = ref("");
const chatContainer = ref<HTMLDivElement | null>(null);
const gateway = useGateway(t);
const chat = useChat(t, gateway);

let messageIdCounter = 1;

const messages = ref<Message[]>([]);

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const canSend = computed(
  () => prompt.value.trim().length > 0 && chat.isSessionReady.value && !chat.sending.value
);
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const showOnboardingGate = computed(() => !chat.isSessionReady.value);

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const combinedErrorMessage = computed(() => {
  const gw = gateway.errorMessage.value;
  const ch = chat.errorMessage.value;
  if (gw && ch) return `${gw} — ${ch}`;
  return ch || gw || "";
});

function createWelcomeMessage(): Message {
  return {
    id: 0,
    role: "assistant",
    content: t("chat.welcome", { modelName }),
  };
}

function resetMessagesForSession(): void {
  messages.value = [createWelcomeMessage()];
}

function nextMessageId(): number {
  const currentId = messageIdCounter;
  messageIdCounter += 1;
  return currentId;
}

function scrollChatToBottom(): void {
  if (!chatContainer.value) {
    return;
  }
  chatContainer.value.scrollTop = chatContainer.value.scrollHeight;
}

function updateAssistantMessage(messageId: number, content: string, status?: MessageStatus): void {
  const messageIndex = messages.value.findIndex((item) => item.id === messageId);
  if (messageIndex >= 0) {
    messages.value[messageIndex] = {
      ...messages.value[messageIndex],
      content,
      status,
    };
  }
}

async function beginSession(preferResume: boolean): Promise<void> {
  const started = chat.startSession(preferResume);
  if (!started) {
    return;
  }

  if (!preferResume) {
    resetMessagesForSession();
  }
  showConfig.value = false;
  await nextTick();
  scrollChatToBottom();
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function startNewSession(): Promise<void> {
  chat.clearSession();
  await beginSession(false);
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function resumeSession(): Promise<void> {
  await beginSession(true);
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function sendMessage(): Promise<void> {
  const text = prompt.value.trim();
  if (!text || !chat.isSessionReady.value) {
    return;
  }

  const normalizedText = text.slice(0, MAX_PROMPT_LENGTH);
  const requestId = gateway.createIdempotencyKey();
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
      gateway: gateway.normalizeBaseUrl(),
    }),
  });

  prompt.value = "";
  await nextTick();
  scrollChatToBottom();

  try {
    // Try streaming first
    updateAssistantMessage(assistantMessageId, "", "streaming");
    await chat.streamMessage(
      normalizedText,
      (chunk) => {
        updateAssistantMessage(assistantMessageId, chunk, "streaming");
        nextTick().then(scrollChatToBottom);
      },
      requestId
    );
    updateAssistantMessage(
      assistantMessageId,
      messages.value.find((m) => m.id === assistantMessageId)?.content ?? "",
      "complete"
    );
  } catch (streamError: unknown) {
    // Rethrow auth/credential errors — do not mask with fallback.
    if (streamError instanceof Error && streamError.message === t("auth.credentialInvalid")) {
      updateAssistantMessage(assistantMessageId, streamError.message, "error");
      await nextTick();
      scrollChatToBottom();
      return;
    }
    // Fall back to non-streaming sendMessage
    try {
      updateAssistantMessage(
        assistantMessageId,
        t("chat.processing", {
          text: normalizedText,
          modelName,
          gateway: gateway.normalizeBaseUrl(),
        }),
        undefined
      );
      const result = await chat.sendMessage(normalizedText, requestId);
      if (result.type === "approval_required") {
        updateAssistantMessage(assistantMessageId, t("chat.toolApprovalTitle"));
        messages.value.push({
          id: nextMessageId(),
          role: "assistant",
          content: "",
          approvalId: result.sessionId,
          toolName: result.tool,
          reason: result.reason,
        });
      } else {
        updateAssistantMessage(assistantMessageId, result.content, "complete");
      }
    } catch (fallbackError) {
      const message =
        fallbackError instanceof Error ? fallbackError.message : t("chat.requestError", { text });
      updateAssistantMessage(assistantMessageId, message, "error");
    }
  }

  await nextTick();
  scrollChatToBottom();
}

watch(
  () => chat.isSessionReady.value,
  (ready) => {
    if (!ready) {
      prompt.value = "";
    }
  }
);

function messagesStorageKey(): string {
  return `corvus-chat-messages-${chat.currentSessionId.value}`;
}

function persistMessages(): void {
  if (!chat.currentSessionId.value) return;
  try {
    const serializable = messages.value.map((m) => ({
      id: m.id,
      role: m.role,
      content: m.content,
      approvalId: m.approvalId,
      toolName: m.toolName,
      reason: m.reason,
    }));
    sessionStorage.setItem(messagesStorageKey(), JSON.stringify(serializable));
  } catch {
    // Ignore storage failures.
  }
}

function restoreMessages(): void {
  if (!chat.currentSessionId.value) return;
  try {
    const raw = sessionStorage.getItem(messagesStorageKey());
    if (!raw) return;
    const parsed = JSON.parse(raw) as Message[];
    if (Array.isArray(parsed) && parsed.length > 0) {
      messages.value = parsed;
      messageIdCounter = Math.max(...parsed.map((m) => m.id)) + 1;
    }
  } catch {
    // Ignore parse failures.
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function handleApprove(approvalId: string): void {
  // Phase 5B stub: full round-trip will be wired in a follow-up.
  const idx = messages.value.findIndex((m) => m.approvalId === approvalId);
  if (idx >= 0) {
    messages.value[idx] = {
      ...messages.value[idx],
      content: t("chat.approve"),
      approvalId: undefined,
      toolName: undefined,
      reason: undefined,
    };
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function handleReject(approvalId: string): void {
  // Phase 5B stub: full round-trip will be wired in a follow-up.
  const idx = messages.value.findIndex((m) => m.approvalId === approvalId);
  if (idx >= 0) {
    messages.value[idx] = {
      ...messages.value[idx],
      content: t("chat.reject"),
      approvalId: undefined,
      toolName: undefined,
      reason: undefined,
    };
  }
}

let persistDebounceTimer: ReturnType<typeof setTimeout> | null = null;

watch(
  () => messages.value,
  () => {
    if (persistDebounceTimer) clearTimeout(persistDebounceTimer);
    persistDebounceTimer = setTimeout(persistMessages, 300);
  },
  { deep: true }
);

watch(
  () => chat.currentSessionId.value,
  (sessionId) => {
    if (sessionId) restoreMessages();
  }
);

onMounted(() => {
  if (chat.currentSessionId.value) restoreMessages();
});

onUnmounted(() => {
  prompt.value = "";
});
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <div class="sidebar-header">
        <div class="logo-icon animate-pulse-glow">
          <img src="/favicon-light.svg" alt="Corvus" width="20" height="20" />
        </div>
        <div>
          <h1 class="sidebar-title">Corvus AI</h1>
          <p class="sidebar-subtitle">{{ t("app.simpleChat") }}</p>
        </div>
      </div>

      <nav class="sidebar-nav">
        <button class="nav-item nav-item--active">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            aria-hidden="true"
          >
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
          </svg>
          {{ t("chat.newChat") }}
        </button>
      </nav>

      <div class="sidebar-footer">
        <button data-testid="toggle-config" class="nav-item" @click="showConfig = !showConfig">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="3" />
            <path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09A1.65 1.65 0 0 0 15 4.6a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9c.23.5.8.83 1.4.83H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09c-.6 0-1.17.33-1.51.83Z"
            />
          </svg>
          {{ showConfig ? t("app.backToChat") : t("app.config") }}
        </button>
      </div>
    </aside>

    <main class="main-content">
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
          :aria-label="showConfig ? t('app.backToChat') : t('app.config')"
          @click="showConfig = !showConfig"
        >
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="3" />
            <path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09A1.65 1.65 0 0 0 15 4.6a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9c.23.5.8.83 1.4.83H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09c-.6 0-1.17.33-1.51.83Z"
            />
          </svg>
        </button>
      </header>

      <div v-if="showConfig" class="config-wrapper">
        <ConfigPanel
          :base-url="gateway.baseUrl.value"
          :pairing-code="gateway.pairingCode.value"
          :bearer-token="gateway.bearerToken.value"
          :webhook-secret="gateway.webhookSecret.value"
          :loading="gateway.loading.value"
          :status-message="gateway.statusMessage.value"
          :error-message="combinedErrorMessage"
          :onboarding-state="gateway.onboardingState.value"
          :onboarding-steps="gateway.onboardingSteps.value"
          @update:base-url="gateway.baseUrl.value = $event"
          @update:pairing-code="gateway.pairingCode.value = $event"
          @update:bearer-token="gateway.bearerToken.value = $event"
          @update:webhook-secret="gateway.webhookSecret.value = $event"
          @pair="gateway.pairGateway"
          @connect="gateway.connectGateway"
        />
      </div>

      <template v-else-if="showOnboardingGate">
        <section class="gate-card">
          <div class="hero-content animate-slide-up">
            <div class="hero-icon animate-pulse-glow">
              <img src="/favicon-light.svg" alt="Corvus" width="32" height="32" />
            </div>
            <p class="hero-kicker">{{ t("sections.auth") }}</p>
            <h2 class="hero-title">{{ t("chatOnboarding.ready.title") }}</h2>
            <p class="hero-subtitle">{{ t("chatOnboarding.intro") }}</p>
          </div>

          <ol class="gate-steps" aria-label="Web chat onboarding steps">
            <li
              v-for="step in gateway.onboardingSteps.value"
              :key="step.key"
              class="gate-step"
              :data-step-status="step.status"
            >
              <div>
                <h3>{{ t(step.titleKey) }}</h3>
                <p>{{ t(step.descriptionKey) }}</p>
              </div>
              <span class="step-badge">{{ t(`onboarding.stepStatus.${step.status}`) }}</span>
            </li>
          </ol>

          <div
            v-if="gateway.onboardingState.value.state === 'blocked' && gateway.onboardingState.value.recoveryKind"
            class="gate-banner gate-banner-error"
            role="alert"
          >
            <p class="banner-title">
              {{ t(`chatOnboarding.recovery.${gateway.onboardingState.value.recoveryKind}.title`) }}
            </p>
            <p>
              {{ t(`chatOnboarding.recovery.${gateway.onboardingState.value.recoveryKind}.description`) }}
            </p>
          </div>

          <div v-else-if="gateway.isGatewayReady.value" class="gate-banner gate-banner-success">
            <p class="banner-title">{{ t("chatOnboarding.session.title") }}</p>
            <p>{{ t("chatOnboarding.session.description") }}</p>
          </div>

          <div class="gate-actions">
            <Button @click="showConfig = true">{{ t("app.config") }}</Button>
            <Button
              v-if="gateway.isGatewayReady.value"
              variant="outline"
              @click="startNewSession"
            >
              {{ t("chat.startSession") }}
            </Button>
            <Button
              v-if="gateway.isGatewayReady.value"
              variant="outline"
              :disabled="!chat.canResumeSession.value"
              @click="resumeSession"
            >
              {{ t("chat.resumeSession") }}
            </Button>
          </div>

          <p v-if="chat.statusMessage.value" class="gate-status gate-status-ok">
            {{ chat.statusMessage.value }}
          </p>
          <p v-if="chat.errorMessage.value" class="gate-status gate-status-error">
            {{ chat.errorMessage.value }}
          </p>
        </section>
      </template>

      <template v-else>
        <div ref="chatContainer" class="chat-messages">
          <div class="chat-messages-inner">
            <template v-for="message in messages" :key="message.id">
              <ToolApprovalCard
                v-if="message.approvalId"
                :tool-name="message.toolName ?? ''"
                :reason="message.reason ?? ''"
                :approval-id="message.approvalId"
                @approve="handleApprove"
                @reject="handleReject"
              />
              <ChatMessage
                v-else
                :role="message.role"
                :content="message.content"
                :status="message.status"
              />
            </template>
          </div>
        </div>

        <div class="chat-toolbar">
          <div class="chat-toolbar-left">
            <p class="session-pill">
              {{ t("chat.sessionActive", { sessionId: chat.currentSessionId.value }) }}
            </p>
            <HealthIndicator
              :gateway-url="gateway.normalizeBaseUrl()"
              :bearer-token="gateway.bearerToken.value"
            />
          </div>
          <Button variant="outline" @click="startNewSession">{{ t("chat.newSession") }}</Button>
        </div>

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
.app-shell {
  display: flex;
  height: 100vh;
  width: 100vw;
  background: radial-gradient(circle at top, rgba(16, 185, 129, 0.08), transparent 36%),
    var(--color-bg-primary);
  overflow: hidden;
}

.sidebar {
  display: none;
  width: 260px;
  flex-direction: column;
  border-right: 1px solid var(--color-border);
  background: color-mix(in srgb, var(--color-bg-secondary) 92%, transparent);
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

.main-content {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}

.mobile-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);
  background: color-mix(in srgb, var(--color-bg-secondary) 94%, transparent);
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

.config-wrapper,
.gate-card {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  padding: 24px;
  overflow-y: auto;
}

.gate-card {
  width: min(760px, 100%);
  flex-direction: column;
  gap: 18px;
}

.hero-content {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  text-align: center;
}

.hero-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 72px;
  height: 72px;
  border-radius: 20px;
  background: var(--color-accent-subtle);
}

.hero-kicker {
  margin: 0;
  font-size: 12px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-text-muted);
}

.hero-title {
  margin: 0;
  font-size: 28px;
  font-weight: 700;
  background: linear-gradient(to right, var(--color-accent), #6ee7b7);
  background-clip: text;
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.hero-subtitle {
  max-width: 580px;
  margin: 0;
  color: var(--color-text-muted);
  line-height: 1.6;
}

.gate-steps {
  width: 100%;
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: 12px;
}

.gate-step {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  border-radius: 16px;
  border: 1px solid var(--color-border);
  background: color-mix(in srgb, var(--color-bg-secondary) 90%, transparent);
  padding: 16px;
}

.gate-step h3,
.gate-step p,
.banner-title,
.gate-banner p,
.session-pill,
.gate-status {
  margin: 0;
}

.gate-step p {
  margin-top: 6px;
  color: var(--color-text-secondary);
}

.gate-step[data-step-status="complete"] {
  border-color: color-mix(in srgb, #22c55e 45%, var(--color-border));
}

.gate-step[data-step-status="current"] {
  border-color: color-mix(in srgb, #3b82f6 45%, var(--color-border));
}

.gate-step[data-step-status="blocked"] {
  border-color: color-mix(in srgb, #ef4444 45%, var(--color-border));
}

.step-badge {
  flex-shrink: 0;
  border-radius: 999px;
  padding: 4px 10px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  background: var(--color-bg-input);
  color: var(--color-text-secondary);
}

.gate-banner {
  width: 100%;
  border-radius: 16px;
  padding: 16px;
}

.gate-banner p:last-child {
  margin-top: 6px;
}

.gate-banner-success {
  border: 1px solid color-mix(in srgb, #22c55e 45%, var(--color-border));
  background: color-mix(in srgb, #22c55e 10%, var(--color-bg-secondary));
}

.gate-banner-error {
  border: 1px solid color-mix(in srgb, #ef4444 45%, var(--color-border));
  background: color-mix(in srgb, #ef4444 10%, var(--color-bg-secondary));
}

.gate-actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: center;
}

.gate-status-ok {
  color: #22c55e;
}

.gate-status-error {
  color: #ef4444;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 24px 16px 8px;
}

@media (min-width: 768px) {
  .chat-messages {
    padding: 24px 32px 8px;
  }
}

@media (min-width: 1024px) {
  .chat-messages {
    padding: 24px 64px 8px;
  }
}

.chat-messages-inner {
  max-width: 768px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.chat-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 0 16px 16px;
}

@media (min-width: 768px) {
  .chat-toolbar {
    padding: 0 32px 16px;
  }
}

@media (min-width: 1024px) {
  .chat-toolbar {
    padding: 0 64px 16px;
  }
}

.chat-toolbar-left {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.session-pill {
  max-width: 768px;
  color: var(--color-text-muted);
  font-size: 12px;
}

.input-bar {
  border-top: 1px solid var(--color-border);
  background: color-mix(in srgb, var(--color-bg-secondary) 94%, transparent);
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

@media (max-width: 767px) {
  .gate-step,
  .chat-toolbar {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
