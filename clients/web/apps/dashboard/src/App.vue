<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { useI18n } from "vue-i18n";

import ChatMessage from "@/components/chat/ChatMessage.vue";
import Button from "@/components/ui/button/Button.vue";
import Input from "@/components/ui/input/Input.vue";
import { escapeHtml } from "@/lib/utils";

type Role = "assistant" | "user";

interface Message {
  id: number;
  role: Role;
  content: string;
}

type SecretField = "pairingCode" | "bearerToken" | "webhookSecret";

const MAX_PROMPT_LENGTH = 500;
const modelName = "Corvus Agent";
const { t } = useI18n();

const showConfig = ref(false);
const prompt = ref("");
const baseUrl = ref("http://127.0.0.1:3000");
const chatContainer = ref<HTMLDivElement | null>(null);
const secretInputNonce = ref(0);

let messageIdCounter = 1;
let pairingCodeInput = "";
let bearerTokenInput = "";
let webhookSecretInput = "";

const messages = ref<Message[]>([
  {
    id: 0,
    role: "assistant",
    content: t("chat.welcome", { modelName }),
  },
]);

const canSend = computed(() => prompt.value.trim().length > 0);

function nextMessageId(): number {
  const currentId = messageIdCounter;
  messageIdCounter += 1;
  return currentId;
}

function captureSecretInput(field: SecretField, value: string): void {
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

function saveGatewayConfig(): void {
  const hasSecretPayload =
    pairingCodeInput.length > 0 || bearerTokenInput.length > 0 || webhookSecretInput.length > 0;
  void hasSecretPayload;

  pairingCodeInput = "";
  bearerTokenInput = "";
  webhookSecretInput = "";
  secretInputNonce.value += 1;
}

function scrollChatToBottom(): void {
  if (!chatContainer.value) {
    return;
  }
  chatContainer.value.scrollTop = chatContainer.value.scrollHeight;
}

async function sendMessage(): Promise<void> {
  const text = prompt.value.trim();
  if (!text) {
    return;
  }

  const normalizedText = text.slice(0, MAX_PROMPT_LENGTH);
  const escapedText = escapeHtml(normalizedText);

  messages.value.push({ id: nextMessageId(), role: "user", content: normalizedText });
  messages.value.push({
    id: nextMessageId(),
    role: "assistant",
    content: t("chat.processing", {
      text: escapedText,
      modelName,
      gateway: baseUrl.value,
    }),
  });

  prompt.value = "";
  await nextTick();
  scrollChatToBottom();
}

void {
  ChatMessage,
  Button,
  Input,
  showConfig,
  prompt,
  baseUrl,
  chatContainer,
  secretInputNonce,
  messages,
  canSend,
  captureSecretInput,
  saveGatewayConfig,
  sendMessage,
};
</script>

<template>
  <main class="min-h-screen bg-slate-50 text-slate-900">
    <section class="mx-auto flex w-full max-w-4xl flex-col gap-4 p-4 md:p-6">
      <header class="flex items-center justify-between rounded-xl border border-slate-200 bg-white p-4">
        <div>
          <h1 class="text-xl font-semibold">{{ modelName }}</h1>
          <p class="text-sm text-slate-500">
            {{ showConfig ? t("app.gatewayConfig") : t("app.simpleChat") }}
          </p>
        </div>
        <Button variant="ghost" size="sm" @click="showConfig = !showConfig">
          {{ showConfig ? t("app.backToChat") : t("app.config") }}
        </Button>
      </header>

      <form
        v-if="showConfig"
        class="grid gap-3 rounded-xl border border-slate-200 bg-white p-4 shadow-sm"
        @submit.prevent="saveGatewayConfig"
      >
        <label class="space-y-1 text-sm">
          <span class="font-medium">{{ t("form.baseUrl") }}</span>
          <Input v-model="baseUrl" :placeholder="t('form.baseUrlPlaceholder')" />
        </label>
        <label class="space-y-1 text-sm">
          <span class="font-medium">{{ t("form.pairingCode") }}</span>
          <Input
            :key="`pairing-${secretInputNonce}`"
            :placeholder="t('form.pairingCodePlaceholder')"
            @update:model-value="(value: string) => captureSecretInput('pairingCode', value)"
          />
        </label>
        <label class="space-y-1 text-sm">
          <span class="font-medium">{{ t("form.bearerToken") }}</span>
          <Input
            :key="`bearer-${secretInputNonce}`"
            :placeholder="t('form.bearerTokenPlaceholder')"
            type="password"
            @update:model-value="(value: string) => captureSecretInput('bearerToken', value)"
          />
        </label>
        <label class="space-y-1 text-sm">
          <span class="font-medium">{{ t("form.webhookSecret") }}</span>
          <Input
            :key="`webhook-${secretInputNonce}`"
            :placeholder="t('form.webhookSecretPlaceholder')"
            type="password"
            @update:model-value="(value: string) => captureSecretInput('webhookSecret', value)"
          />
        </label>
        <div class="flex justify-end">
          <Button type="submit">{{ t("form.save") }}</Button>
        </div>
      </form>

      <section v-else class="flex min-h-[70vh] flex-col rounded-xl border border-slate-200 bg-white shadow-sm">
        <div ref="chatContainer" class="flex-1 space-y-3 overflow-y-auto p-4">
          <ChatMessage
            v-for="message in messages"
            :key="message.id"
            :role="message.role"
            :content="message.content"
          />
        </div>

        <form class="flex gap-2 border-t border-slate-200 p-3" @submit.prevent="sendMessage">
          <Input v-model="prompt" :placeholder="t('chat.inputPlaceholder')" />
          <Button type="submit" :disabled="!canSend">{{ t("chat.send") }}</Button>
        </form>
      </section>
    </section>
  </main>
</template>
