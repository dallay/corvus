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
    :class="['flex w-full', role === 'user' ? 'justify-end' : 'justify-start']"
  >
    <div
      :class="[
        'max-w-[80%] rounded-xl px-4 py-3 text-sm shadow-sm',
        role === 'user' ? 'bg-slate-900 text-slate-50' : 'bg-slate-100 text-slate-900',
      ]"
    >
      {{ content }}
    </div>
  </div>
</template>
