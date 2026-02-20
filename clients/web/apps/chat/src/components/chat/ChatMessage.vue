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
    <div v-if="role === 'assistant'" class="avatar avatar--assistant">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 2L2 7l10 5 10-5-10-5z" />
        <path d="M2 17l10 5 10-5" />
        <path d="M2 12l10 5 10-5" />
      </svg>
    </div>

    <!-- Bubble -->
    <div :class="['bubble', role === 'user' ? 'bubble--user' : 'bubble--assistant']">
      <p class="bubble-text">{{ content }}</p>
    </div>

    <!-- User Avatar -->
    <div v-if="role === 'user'" class="avatar avatar--user">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
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
  border-radius: 8px;
  flex-shrink: 0;
  margin-top: 4px;
}

.avatar--assistant {
  margin-right: 12px;
  background: var(--color-accent-subtle);
  color: var(--color-accent);
}

.avatar--user {
  margin-left: 12px;
  background: var(--color-bg-hover);
  color: var(--color-text-secondary);
}

.bubble {
  max-width: 75%;
  padding: 12px 16px;
  font-size: 14px;
  line-height: 1.6;
}

.bubble--user {
  border-radius: 16px 16px 4px 16px;
  background: var(--color-user-bubble);
  color: var(--color-user-bubble-text);
  box-shadow: 0 4px 12px var(--color-accent-glow);
}

.bubble--assistant {
  border-radius: 16px 16px 16px 4px;
  background: var(--color-assistant-bubble);
  color: var(--color-assistant-bubble-text);
  border: 1px solid var(--color-border);
}

.bubble-text {
  margin: 0;
  white-space: pre-wrap;
}

@keyframes fade-in {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
