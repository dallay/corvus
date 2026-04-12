<script setup lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import { Button } from "@corvus/ui";
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import ChatMessage from "@/components/chat/ChatMessage.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import HealthIndicator from "@/components/chat/HealthIndicator.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import SessionSidebar from "@/components/chat/SessionSidebar.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import ToolApprovalCard from "@/components/chat/ToolApprovalCard.vue";
import { useChat } from "@/composables/useChat";
import { useChatGateway } from "@/composables/useChatGateway";
import type { useConfig } from "@/composables/useConfig";

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
  recalledMemoryKeys?: string[];
}

const MAX_PROMPT_LENGTH = 500;
const modelName = "Corvus Agent";

const props = defineProps<{
  config: ReturnType<typeof useConfig>;
}>();

const { t } = useI18n();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const sidebarCollapsed = ref(true);
const prompt = ref("");
const chatContainer = ref<HTMLDivElement | null>(null);
const promptInputRef = ref<HTMLInputElement | null>(null);
const sessionAnnouncement = ref("");
const approvalAnnouncement = ref("");

const gateway = useChatGateway(props.config, t);
const chat = useChat(t, gateway);

let messageIdCounter = 1;

const messages = ref<Message[]>([]);

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const canSend = computed(
  () => prompt.value.trim().length > 0 && chat.isSessionReady.value && !chat.sending.value
);
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const showOnboardingGate = computed(() => !chat.isSessionReady.value);

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

function queueA11yAnnouncement(message: string): void {
  sessionAnnouncement.value = "";
  globalThis.setTimeout(() => {
    sessionAnnouncement.value = message;
  }, 0);
}

function queueApprovalAnnouncement(message: string): void {
  approvalAnnouncement.value = "";
  globalThis.setTimeout(() => {
    approvalAnnouncement.value = message;
  }, 0);
}

async function focusPromptInput(): Promise<void> {
  await nextTick();
  promptInputRef.value?.focus();
}

function updateAssistantMessage(
  messageId: number,
  content: string,
  status?: MessageStatus,
  recalledMemoryKeys?: string[]
): void {
  const messageIndex = messages.value.findIndex((item) => item.id === messageId);
  if (messageIndex >= 0) {
    messages.value[messageIndex] = {
      ...messages.value[messageIndex],
      content,
      status,
      ...(recalledMemoryKeys != null && { recalledMemoryKeys }),
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
  await nextTick();
  scrollChatToBottom();
  await focusPromptInput();
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function startNewSession(): Promise<void> {
  persistMessages();
  chat.clearSession();
  await beginSession(false);
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function resumeSession(): Promise<void> {
  await beginSession(true);
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function handleSidebarNewChat(): Promise<void> {
  persistMessages();
  chat.clearSession();
  await beginSession(false);
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function handleSwitchSession(targetSessionId: string): void {
  persistMessages();
  chat.switchSession(targetSessionId);
  void focusPromptInput();
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
    let streamBuffer = "";
    updateAssistantMessage(assistantMessageId, "", "streaming");
    const doneEvent = await chat.streamMessage(
      normalizedText,
      (chunk) => {
        streamBuffer += chunk;
        updateAssistantMessage(assistantMessageId, streamBuffer, "streaming");
        nextTick().then(scrollChatToBottom);
      },
      requestId
    );
    updateAssistantMessage(
      assistantMessageId,
      streamBuffer,
      "complete",
      doneEvent.recalled_memory_keys
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
          approvalId: gateway.createIdempotencyKey(),
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
  return `corvus-chat-messages-${encodeURIComponent(gateway.normalizeBaseUrl())}:${chat.currentSessionId.value}`;
}

function persistMessages(): void {
  if (!chat.currentSessionId.value) return;
  try {
    const serializable = messages.value.map((m) => ({
      id: m.id,
      role: m.role,
      content: m.content,
      status: m.status,
      approvalId: m.approvalId,
      toolName: m.toolName,
      reason: m.reason,
      recalledMemoryKeys: m.recalledMemoryKeys,
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
    if (!raw) {
      resetMessagesForSession();
      return;
    }
    const parsed = JSON.parse(raw) as unknown[];
    if (!Array.isArray(parsed) || parsed.length === 0) {
      resetMessagesForSession();
      return;
    }
    const validStatuses: MessageStatus[] = ["streaming", "complete", "error"];
    const validRoles: Role[] = ["assistant", "user"];
    const isValidMessage = (value: unknown): value is Message => {
      if (value === null || typeof value !== "object") {
        return false;
      }

      const message = value as Record<string, unknown>;
      const id = message.id;
      return (
        typeof id === "number" &&
        Number.isInteger(id) &&
        Number.isFinite(id) &&
        validRoles.includes(message.role as Role) &&
        typeof message.content === "string" &&
        (message.status === undefined || validStatuses.includes(message.status as MessageStatus)) &&
        (message.approvalId === undefined || typeof message.approvalId === "string") &&
        (message.toolName === undefined || typeof message.toolName === "string") &&
        (message.reason === undefined || typeof message.reason === "string") &&
        (message.recalledMemoryKeys === undefined ||
          (Array.isArray(message.recalledMemoryKeys) &&
            message.recalledMemoryKeys.every((k) => typeof k === "string")))
      );
    };

    if (parsed.every(isValidMessage)) {
      messages.value = parsed;
      messageIdCounter = Math.max(...parsed.map((m) => m.id)) + 1;
    } else {
      resetMessagesForSession();
    }
  } catch {
    resetMessagesForSession();
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
    queueApprovalAnnouncement(t("chat.approve"));
    void focusPromptInput();
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
    queueApprovalAnnouncement(t("chat.reject"));
    void focusPromptInput();
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
  (sessionId, previousSessionId) => {
    if (sessionId) {
      restoreMessages();
      queueA11yAnnouncement(t("chat.sessionActive", { sessionId }));
      if (sessionId !== previousSessionId) {
        void focusPromptInput();
      }
    } else {
      sessionAnnouncement.value = "";
      approvalAnnouncement.value = "";
    }
  }
);

watch(
  () => gateway.bearerToken.value,
  () => {
    if (chat.currentSessionId.value) {
      try {
        sessionStorage.removeItem(messagesStorageKey());
      } catch {
        // Ignore storage failures.
      }
      resetMessagesForSession();
    }
  }
);

onMounted(() => {
  if (chat.currentSessionId.value) restoreMessages();
});

onUnmounted(() => {
  prompt.value = "";
  chat.stopSessionPolling();
});
</script>

<template>
  <div class="chat-workspace">
    <template v-if="showOnboardingGate">
      <section class="chat-gate">
        <div class="chat-gate-inner">
          <div class="chat-gate-icon">
            <img src="/favicon-light.svg" alt="Corvus" width="32" height="32" />
          </div>
          <h2 class="chat-gate-title">{{ t("chatOnboarding.session.title") }}</h2>
          <p class="chat-gate-copy">{{ t("chatOnboarding.session.description") }}</p>

          <div class="chat-gate-actions">
            <Button
              v-if="config.isOperatorReady.value"
              @click="startNewSession"
            >
              {{ t("chat.startSession") }}
            </Button>
            <Button
              v-if="config.isOperatorReady.value"
              variant="secondary"
              :disabled="!chat.canResumeSession.value"
              @click="resumeSession"
            >
              {{ t("chat.resumeSession") }}
            </Button>
          </div>

          <p v-if="chat.statusMessage.value" aria-live="polite" class="chat-gate-status chat-gate-status--ok">
            {{ chat.statusMessage.value }}
          </p>
          <p v-if="chat.errorMessage.value" aria-live="assertive" class="chat-gate-status chat-gate-status--error">
            {{ chat.errorMessage.value }}
          </p>
        </div>
      </section>
    </template>

    <template v-else>
      <div class="chat-with-sidebar">
        <SessionSidebar
          :sessions="chat.sessionList.value"
          :current-session-id="chat.currentSessionId.value"
          :collapsed="sidebarCollapsed"
          @switch-session="handleSwitchSession"
          @new-chat="handleSidebarNewChat"
          @toggle-collapse="sidebarCollapsed = !sidebarCollapsed"
        />
        <div class="chat-viewport">
          <div aria-atomic="true" aria-live="polite" class="sr-only" role="status">
            {{ sessionAnnouncement }}
          </div>
          <div aria-atomic="true" aria-live="polite" class="sr-only" role="status">
            {{ approvalAnnouncement }}
          </div>
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
                  :recalled-memory-keys="message.recalledMemoryKeys"
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
            <Button variant="secondary" @click="startNewSession">{{ t("chat.newSession") }}</Button>
          </div>

          <div class="input-bar">
            <form class="input-bar-inner" @submit.prevent="sendMessage">
              <div class="input-wrapper">
                <label class="sr-only" for="chat-prompt-input">{{ t("chat.inputPlaceholder") }}</label>
                <input
                  id="chat-prompt-input"
                  ref="promptInputRef"
                  v-model="prompt"
                  :aria-label="t('chat.inputPlaceholder')"
                  aria-describedby="chat-input-disclaimer"
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
            <p id="chat-input-disclaimer" class="input-disclaimer">{{ t("chat.disclaimer") }}</p>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.chat-workspace {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  border: 1px solid var(--corvus-color-border-default);
  border-radius: var(--corvus-radius-card-lg);
  background: var(--corvus-color-bg-base);
  overflow: hidden;
}

.chat-gate {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  padding: 32px 24px;
}

.chat-gate-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  max-width: 440px;
  text-align: center;
}

.chat-gate-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 64px;
  height: 64px;
  border-radius: 16px;
  background: var(--corvus-color-accent-subtle);
}

.chat-gate-title {
  margin: 0;
  font-size: 22px;
  letter-spacing: -0.02em;
}

.chat-gate-copy {
  margin: 0;
  color: var(--corvus-color-text-secondary);
  font-size: 14px;
  line-height: 1.5;
}

.chat-gate-actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: center;
}

.chat-gate-status {
  margin: 0;
  font-size: 13px;
}

.chat-gate-status--ok {
  color: var(--corvus-color-status-success);
}

.chat-gate-status--error {
  color: var(--corvus-color-status-error);
}

.sr-only {
  border: 0;
  clip: rect(0, 0, 0, 0);
  height: 1px;
  margin: -1px;
  overflow: hidden;
  padding: 0;
  position: absolute;
  white-space: nowrap;
  width: 1px;
}

.chat-with-sidebar {
  display: flex;
  flex: 1;
  min-height: 0;
}

.chat-viewport {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
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

.chat-toolbar-left {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.session-pill {
  max-width: 768px;
  color: var(--corvus-color-text-disabled);
  font-size: 12px;
  margin: 0;
}

.input-bar {
  border-top: 1px solid var(--corvus-color-border-default);
  background: var(--corvus-color-bg-surface);
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
  border-radius: var(--corvus-radius-card-lg);
  border: 1px solid var(--corvus-color-border-default);
  background: var(--corvus-color-bg-surface);
  padding: 0 16px;
  transition: border-color var(--corvus-motion-duration-default) var(--corvus-motion-easing-default);
}

.input-wrapper:focus-within {
  border-color: var(--corvus-color-text-primary);
}

.chat-input {
  flex: 1;
  background: transparent;
  border: none;
  padding: 12px 0;
  font-size: 14px;
  font-family: inherit;
  color: var(--corvus-color-text-primary);
  outline: none;
}

.chat-input::placeholder {
  color: var(--corvus-color-text-disabled);
}

.send-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: var(--corvus-radius-card-lg);
  border: none;
  background: var(--corvus-color-text-display);
  color: var(--corvus-color-bg-base);
  cursor: pointer;
  transition: background var(--corvus-motion-duration-default) var(--corvus-motion-easing-default);
  flex-shrink: 0;
}

.send-btn:hover:not(:disabled) {
  background: var(--corvus-color-text-primary);
}

.send-btn:focus-visible:not(:disabled) {
  outline: 2px solid var(--corvus-color-text-primary);
  outline-offset: 2px;
}

.send-btn:active:not(:disabled) {
  transform: scale(0.95);
}

.send-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.input-disclaimer {
  margin: 8px 0 0;
  text-align: center;
  font-size: 12px;
  color: var(--corvus-color-text-disabled);
}

@media (max-width: 767px) {
  .chat-toolbar {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
