<script setup lang="ts">
import { onMounted, ref } from "vue";

defineProps<{
  toolName: string;
  reason: string;
  approvalId: string;
}>();

const emit = defineEmits<{
  approve: [approvalId: string];
  reject: [approvalId: string];
}>();

const isSubmitting = ref(false);
const approveButtonRef = ref<HTMLButtonElement | null>(null);

function focusPrimaryAction(): void {
  approveButtonRef.value?.focus();
}

defineExpose({
  focusPrimaryAction,
});

onMounted(() => {
  void focusPrimaryAction();
});

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function handleApprove(id: string): void {
  if (isSubmitting.value) return;
  isSubmitting.value = true;
  emit("approve", id);
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function handleReject(id: string): void {
  if (isSubmitting.value) return;
  isSubmitting.value = true;
  emit("reject", id);
}
</script>

<template>
  <fieldset
    :aria-describedby="`tool-approval-reason-${approvalId}`"
    class="tool-approval-card"
    data-testid="tool-approval"
  >
    <legend class="tool-name" data-testid="tool-name">{{ toolName }}</legend>
    <div class="tool-approval-header">
      <span class="tool-icon">🔧</span>
    </div>
    <p :id="`tool-approval-reason-${approvalId}`" class="tool-reason" data-testid="tool-reason">{{ reason }}</p>
    <div class="tool-approval-actions">
      <button ref="approveButtonRef" type="button" class="btn-approve touch-target" data-testid="btn-approve" :disabled="isSubmitting" @click="handleApprove(approvalId)">
        {{ $t("chat.approve") }}
      </button>
      <button type="button" class="btn-reject touch-target" data-testid="btn-reject" :disabled="isSubmitting" @click="handleReject(approvalId)">
        {{ $t("chat.reject") }}
      </button>
    </div>
  </fieldset>
</template>

<style scoped>
.tool-approval-card {
  border-radius: var(--corvus-radius-card-lg);
  border: 1px solid var(--corvus-color-status-warning);
  background: var(--corvus-color-bg-surface);
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
  padding: 0;
  font-weight: 600;
  font-size: 14px;
  color: var(--corvus-color-text-primary);
}

.tool-reason {
  margin: 0;
  font-size: 14px;
  line-height: 1.5;
  color: var(--corvus-color-text-secondary);
}

.tool-approval-actions {
  display: flex;
  gap: 8px;
}

.touch-target {
  min-height: 32px;
  min-width: 32px;
}

.btn-approve,
.btn-reject {
  padding: 6px 16px;
  border-radius: var(--corvus-radius-input);
  font-size: 13px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all var(--corvus-motion-duration-default) var(--corvus-motion-easing-default);
}

.btn-approve {
  background: transparent;
  color: var(--corvus-color-status-success);
  border-color: var(--corvus-color-status-success);
}

.btn-approve:hover {
  background: var(--corvus-color-bg-raised);
}

.btn-reject {
  background: transparent;
  color: var(--corvus-color-status-error);
  border-color: var(--corvus-color-status-error);
}

.btn-reject:hover {
  background: var(--corvus-color-bg-raised);
}
</style>
