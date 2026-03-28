<script setup lang="ts">
defineProps<{
  toolName: string;
  reason: string;
  approvalId: string;
}>();

defineEmits<{
  approve: [approvalId: string];
  reject: [approvalId: string];
}>();
</script>

<template>
  <div class="tool-approval-card" data-testid="tool-approval">
    <div class="tool-approval-header">
      <span class="tool-icon">🔧</span>
      <span class="tool-name" data-testid="tool-name">{{ toolName }}</span>
    </div>
    <p class="tool-reason" data-testid="tool-reason">{{ reason }}</p>
    <div class="tool-approval-actions">
      <button class="btn-approve" data-testid="btn-approve" @click="$emit('approve', approvalId)">
        {{ $t("chat.approve") }}
      </button>
      <button class="btn-reject" data-testid="btn-reject" @click="$emit('reject', approvalId)">
        {{ $t("chat.reject") }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.tool-approval-card {
  border-radius: 16px;
  border: 1px solid color-mix(in srgb, #f59e0b 45%, var(--color-border));
  background: color-mix(in srgb, #f59e0b 8%, var(--color-bg-secondary));
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-width: 75%;
  animation: fade-in 0.3s ease-out forwards;
}

.tool-approval-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tool-icon {
  font-size: 16px;
}

.tool-name {
  font-weight: 600;
  font-size: 14px;
  color: var(--color-text-primary);
}

.tool-reason {
  margin: 0;
  font-size: 14px;
  line-height: 1.5;
  color: var(--color-text-secondary);
}

.tool-approval-actions {
  display: flex;
  gap: 8px;
}

.btn-approve,
.btn-reject {
  padding: 6px 16px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.2s;
}

.btn-approve {
  background: color-mix(in srgb, #22c55e 20%, var(--color-bg-secondary));
  color: #22c55e;
  border-color: color-mix(in srgb, #22c55e 40%, var(--color-border));
}

.btn-approve:hover {
  background: color-mix(in srgb, #22c55e 30%, var(--color-bg-secondary));
}

.btn-reject {
  background: color-mix(in srgb, #ef4444 20%, var(--color-bg-secondary));
  color: #ef4444;
  border-color: color-mix(in srgb, #ef4444 40%, var(--color-border));
}

.btn-reject:hover {
  background: color-mix(in srgb, #ef4444 30%, var(--color-bg-secondary));
}
</style>
