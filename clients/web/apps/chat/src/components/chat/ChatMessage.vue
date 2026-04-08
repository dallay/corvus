<script setup lang="ts">
defineProps<{
  role: "assistant" | "user";
  content: string;
  status?: "streaming" | "complete" | "error";
}>();
</script>

<template>
  <div
    data-testid="chat-message"
    :data-role="role"
    :data-status="status"
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
    <div
      :class="[
        'bubble',
        role === 'user' ? 'bubble--user' : 'bubble--assistant',
        status === 'error' ? 'bubble--error' : '',
      ]"
      :aria-live="role === 'assistant' && status === 'streaming' ? 'polite' : undefined"
    >
      <p class="bubble-text">{{ content }}<span v-if="status === 'streaming'" class="streaming-cursor" aria-hidden="true"></span></p>
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
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--corvus-radius-input);
  flex-shrink: 0;
  margin-top: 4px;
}

.avatar--assistant {
  margin-right: 12px;
  background: var(--corvus-color-accent-subtle);
  color: var(--corvus-color-accent-default);
}

.avatar--user {
  margin-left: 12px;
  background: var(--corvus-color-bg-raised);
  color: var(--corvus-color-text-secondary);
}

.bubble {
  max-width: 75%;
  padding: 12px 16px;
  font-size: 14px;
  line-height: 1.6;
}

.bubble--user {
  border-radius: var(--corvus-radius-card-lg) var(--corvus-radius-card-lg) var(--corvus-radius-technical) var(--corvus-radius-card-lg);
  background: var(--corvus-color-bg-raised);
  color: var(--corvus-color-text-primary);
  border: 1px solid var(--corvus-color-border-visible);
}

.bubble--assistant {
  border-radius: var(--corvus-radius-card-lg) var(--corvus-radius-card-lg) var(--corvus-radius-card-lg) var(--corvus-radius-technical);
  background: var(--corvus-color-bg-surface);
  color: var(--corvus-color-text-primary);
  border: 1px solid var(--corvus-color-border-default);
}

.bubble--error {
  border-color: var(--corvus-color-status-error);
  background: var(--corvus-color-bg-surface);
}

.bubble-text {
  margin: 0;
  white-space: pre-wrap;
}

.streaming-cursor {
  display: inline-block;
  width: 6px;
  height: 14px;
  margin-left: 2px;
  vertical-align: text-bottom;
  background: var(--corvus-color-text-primary);
  border-radius: 1px;
  animation: cursor-blink 0.8s step-end infinite;
}

@keyframes cursor-blink {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0;
  }
}
</style>
