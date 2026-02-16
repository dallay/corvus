<script setup lang="ts">
import { ref, onMounted, nextTick } from "vue";
import { Send, User, Bot } from "lucide-vue-next";
import Button from "@/components/ui/Button.vue";
import Input from "@/components/ui/Input.vue";

interface Message {
  id: number;
  text: string;
  isBot: boolean;
}

const messages = ref<Message[]>([
  { id: 1, text: "Hola! Soy el asistente de Corvus. ¿En qué puedo ayudarte hoy?", isBot: true },
]);

const newMessage = ref("");
const scrollContainer = ref<HTMLElement | null>(null);

const scrollToBottom = async () => {
  await nextTick();
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight;
  }
};

const sendMessage = () => {
  if (!newMessage.value.trim()) return;

  messages.value.push({
    id: Date.now(),
    text: newMessage.value,
    isBot: false,
  });

  const userText = newMessage.value;
  newMessage.value = "";
  scrollToBottom();

  // Simulate bot response
  setTimeout(() => {
    messages.value.push({
      id: Date.now(),
      text: `Recibí tu mensaje: "${userText}". Soy un demo de interfaz de chat construido con Vue.js y shadcn-vue.`,
      isBot: true,
    });
    scrollToBottom();
  }, 1000);
};

onMounted(() => {
  scrollToBottom();
});
</script>

<template>
  <div class="flex flex-col h-screen bg-background text-foreground max-w-2xl mx-auto border-x">
    <!-- Header -->
    <header class="border-b p-4 flex items-center justify-between bg-card">
      <div class="flex items-center gap-2">
        <div class="bg-primary p-2 rounded-lg">
          <Bot class="text-primary-foreground" :size="24" />
        </div>
        <div>
          <h1 class="text-lg font-bold leading-none">Corvus Assistant</h1>
          <span class="text-xs text-muted-foreground">Siempre activo</span>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <div class="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
        <span class="text-xs font-medium">Online</span>
      </div>
    </header>

    <!-- Chat Area -->
    <main
      ref="scrollContainer"
      class="flex-1 overflow-y-auto p-4 space-y-4 scroll-smooth"
    >
      <div
        v-for="message in messages"
        :key="message.id"
        :class="['flex w-full', message.isBot ? 'justify-start' : 'justify-end']"
      >
        <div
          :class="[
            'max-w-[85%] rounded-2xl p-4 shadow-sm transition-all',
            message.isBot
              ? 'bg-secondary text-secondary-foreground rounded-tl-none'
              : 'bg-primary text-primary-foreground rounded-tr-none'
          ]"
        >
          <div class="flex items-start gap-3">
            <div v-if="message.isBot" class="shrink-0 mt-0.5">
              <Bot :size="16" />
            </div>
            <div class="flex flex-col gap-1">
              <p class="text-sm leading-relaxed">{{ message.text }}</p>
              <span class="text-[10px] opacity-70 self-end">
                {{ new Date(message.id).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }}
              </span>
            </div>
            <div v-if="!message.isBot" class="shrink-0 mt-0.5">
              <User :size="16" />
            </div>
          </div>
        </div>
      </div>
    </main>

    <!-- Footer / Input -->
    <footer class="border-t p-4 bg-card">
      <form @submit.prevent="sendMessage" class="flex gap-2 items-center">
        <Input
          v-model="newMessage"
          placeholder="Escribe un mensaje..."
          class="flex-1"
          @keydown.enter.prevent="sendMessage"
        />
        <Button type="submit" size="icon" :disabled="!newMessage.trim()">
          <Send :size="18" />
        </Button>
      </form>
      <p class="text-[10px] text-center text-muted-foreground mt-2">
        Powered by Corvus AI • Vue.js & shadcn-vue
      </p>
    </footer>
  </div>
</template>

<style>
/* Custom scrollbar for better look */
::-webkit-scrollbar {
  width: 6px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: hsl(var(--muted));
  border-radius: 10px;
}
::-webkit-scrollbar-thumb:hover {
  background: hsl(var(--muted-foreground) / 0.5);
}
</style>
