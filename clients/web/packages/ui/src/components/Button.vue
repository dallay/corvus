<script setup lang="ts">
defineOptions({ inheritAttrs: false });

const props = defineProps<{
  variant?: "default" | "ghost" | "outline";
  size?: "default" | "sm" | "lg" | "icon";
  type?: "button" | "submit" | "reset";
}>();
</script>

<template>
  <button
    v-bind="$attrs"
    :type="type ?? 'button'"
    :class="[
      'btn',
      `btn--${props.variant ?? 'default'}`,
      `btn-size--${props.size ?? 'default'}`,
      ($attrs.class as string),
    ]"
  >
    <slot />
  </button>
</template>

<style scoped>
.btn {
  display: inline-flex;
  gap: 8px;
  align-items: center;
  justify-content: center;
  font-family: inherit;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  user-select: none;
  outline: none;
  border: none;
  border-radius: 12px;
  transition: all 0.2s;
}

.btn:disabled {
  pointer-events: none;
  cursor: not-allowed;
  opacity: 0.4;
}

.btn:focus-visible {
  box-shadow: 0 0 0 2px var(--color-bg-primary), 0 0 0 4px var(--color-accent);
}

/* Variants */
.btn--default {
  color: white;
  background: var(--color-accent);
  box-shadow: 0 4px 12px var(--color-accent-glow);
}

.btn--default:hover:not(:disabled) {
  background: var(--color-accent-hover);
  box-shadow: 0 6px 20px var(--color-accent-glow);
}

.btn--default:active:not(:disabled) {
  transform: scale(0.97);
}

.btn--ghost {
  color: var(--color-text-secondary);
  background: transparent;
}

.btn--ghost:hover:not(:disabled) {
  color: var(--color-text-primary);
  background: var(--color-surface-glass-hover);
}

.btn--outline {
  color: var(--color-text-secondary);
  background: transparent;
  border: 1px solid var(--color-border);
}

.btn--outline:hover:not(:disabled) {
  color: var(--color-text-primary);
  background: var(--color-surface-glass);
  border-color: var(--color-border-hover);
}

/* Sizes */
.btn-size--default {
  height: 40px;
  padding: 0 20px;
}

.btn-size--sm {
  height: 32px;
  padding: 0 12px;
  font-size: 12px;
}

.btn-size--lg {
  height: 48px;
  padding: 0 24px;
  font-size: 16px;
}

.btn-size--icon {
  width: 36px;
  height: 36px;
  padding: 0;
}
</style>
