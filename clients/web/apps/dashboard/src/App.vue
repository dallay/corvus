<script setup lang="ts">
import { computed, ref } from "vue";

type Role = "assistant" | "user";

interface Message {
  id: number;
  role: Role;
  content: string;
}

const modelName = "Corvus Agent";
const _showConfig = ref(false);
const prompt = ref("");
const baseUrl = ref("http://127.0.0.1:3000");
const _pairingCode = ref("");
const _bearerToken = ref("");
const _webhookSecret = ref("");

const messages = ref<Message[]>([
  {
    id: 0,
    role: "assistant",
    content: `Hola, soy ${modelName}. ¿En qué puedo ayudarte?`,
  },
]);

const _canSend = computed(() => prompt.value.trim().length > 0);

function _sendMessage() {
  const text = prompt.value.trim();
  if (!text) {
    return;
  }

  messages.value.push({ id: Date.now(), role: "user", content: text });
  messages.value.push({
    id: Date.now() + 1,
    role: "assistant",
    content: `Procesando "${text}" con ${modelName}. Gateway: ${baseUrl.value}`,
  });
  prompt.value = "";
}
</script>

<template>
  <main class="min-h-screen bg-slate-50 text-slate-900">
    <section class="mx-auto flex w-full max-w-4xl flex-col gap-4 p-4 md:p-6">
      <header class="flex items-center justify-between rounded-xl border border-slate-200 bg-white p-4">
        <div>
          <h1 class="text-xl font-semibold">{{ modelName }}</h1>
          <p class="text-sm text-slate-500">
            {{ showConfig ? 'Configuración del gateway' : 'Simple AI chat' }}
          </p>
        </div>
        <Button variant="ghost" size="sm" @click="showConfig = !showConfig">
          {{ showConfig ? 'Volver al chat' : 'Config' }}
        </Button>
      </header>

      <section
        v-if="showConfig"
        class="grid gap-3 rounded-xl border border-slate-200 bg-white p-4 shadow-sm"
      >
        <label class="space-y-1 text-sm">
          <span class="font-medium">Base URL</span>
          <Input v-model="baseUrl" placeholder="http://127.0.0.1:3000" />
        </label>
        <label class="space-y-1 text-sm">
          <span class="font-medium">Pairing code</span>
          <Input v-model="pairingCode" placeholder="Pairing code" />
        </label>
        <label class="space-y-1 text-sm">
          <span class="font-medium">Bearer token</span>
          <Input v-model="bearerToken" placeholder="Bearer token" type="password" />
        </label>
        <label class="space-y-1 text-sm">
          <span class="font-medium">Webhook secret</span>
          <Input v-model="webhookSecret" placeholder="Webhook secret" type="password" />
        </label>
      </section>

      <section v-else class="flex min-h-[70vh] flex-col rounded-xl border border-slate-200 bg-white shadow-sm">
        <div class="flex-1 space-y-3 overflow-y-auto p-4">
          <ChatMessage
            v-for="message in messages"
            :key="message.id"
            :role="message.role"
            :content="message.content"
          />
        </div>

        <form class="flex gap-2 border-t border-slate-200 p-3" @submit.prevent="sendMessage">
          <Input v-model="prompt" placeholder="Escribe un mensaje..." />
          <Button type="submit" :disabled="!canSend">Enviar</Button>
        </form>
      </section>
    </section>
  </main>
</template>
