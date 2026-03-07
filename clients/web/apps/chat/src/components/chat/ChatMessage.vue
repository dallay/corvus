<script setup lang="ts">
defineProps<{
  role: "assistant" | "user";
  content: string;
}>();
</script>

<template>
  <div
    data-testid="chat-message"
    :data-role="role"
    :class="['message-row', role === 'user' ? 'message-row--user' : 'message-row--assistant']"
  >
    <!-- Assistant Avatar -->
    <div v-if="role === 'assistant'" class="avatar avatar--assistant" aria-hidden="true">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M12 3 4.5 7.5V16.5L12 21 19.5 16.5V7.5L12 3Z" />
        <path d="M12 12 19.5 7.5" />
        <path d="M12 12V21" />
        <path d="M12 12 4.5 7.5" />
      </svg>
    </div>

    <!-- Bubble -->
    <div :class="['bubble', role === 'user' ? 'bubble--user' : 'bubble--assistant']">
      <p class="bubble-text">{{ content }}</p>
    </div>

    <!-- User Avatar -->
    <div v-if="role === 'user'" class="avatar avatar--user" aria-hidden="true">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
        <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
        <circle cx="12" cy="7" r="4" />
      </svg>
    </div>
  </div>
</template>

<style scoped>
.message-row {
  display: flex;
  width: 100%;
  animation: fade-in 0.3s ease-out forwards;
}

.message-row--user {
  justify-content: flex-end;
}

.message-row--assistant {
  justify-content: flex-start;
}

.avatar {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  margin-top: 4px;
  border-radius: 8px;
}

.avatar--assistant {
  margin-right: 12px;
  color: var(--color-accent);
  background: var(--color-accent-subtle);
}

.avatar--user {
  margin-left: 12px;
  color: var(--color-text-secondary);
  background: var(--color-bg-hover);
}

.bubble {
  max-width: 75%;
  padding: 12px 16px;
  font-size: 14px;
  line-height: 1.6;
}

.bubble--user {
  color: var(--color-user-bubble-text);
  background: var(--color-user-bubble);
  border-radius: 16px 16px 4px;
  box-shadow: 0 4px 12px var(--color-accent-glow);
}

.bubble--assistant {
  color: var(--color-assistant-bubble-text);
  background: var(--color-assistant-bubble);
  border: 1px solid var(--color-border);
  border-radius: 16px 16px 16px 4px;
}

.bubble-text {
  margin: 0;
  white-space: pre-wrap;
}
</style>
