<script setup lang="ts">
const props = defineProps<{
  class?: string;
  variant?: "default" | "ghost" | "outline";
  size?: "default" | "sm" | "lg" | "icon";
  type?: "button" | "submit" | "reset";
}>();
</script>

<template>
  <button
    :type="type ?? 'button'"
    :class="[
      'btn',
      `btn--${props.variant ?? 'default'}`,
      `btn--${props.size ?? 'default'}`,
      props.class,
    ]"
  >
    <slot />
  </button>
</template>

<style scoped>
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 500;
  font-family: inherit;
  border: none;
  cursor: pointer;
  transition: all 0.2s;
  user-select: none;
  outline: none;
}

.btn:disabled {
  opacity: 0.4;
  pointer-events: none;
}

.btn:focus-visible {
  box-shadow: 0 0 0 2px var(--color-bg-primary), 0 0 0 4px var(--color-accent);
}

/* Variants */

.btn--default {
  background: var(--color-accent);
  color: white;
  box-shadow: 0 4px 12px var(--color-accent-glow);
}

.btn--default:hover {
  background: var(--color-accent-hover);
  box-shadow: 0 6px 20px var(--color-accent-glow);
}

.btn--default:active {
  transform: scale(0.97);
}

.btn--ghost {
  background: transparent;
  color: var(--color-text-secondary);
}

.btn--ghost:hover {
  color: var(--color-text-primary);
  background: var(--color-surface-glass-hover);
}

.btn--outline {
  background: transparent;
  border: 1px solid var(--color-border);
  color: var(--color-text-secondary);
}

.btn--outline:hover {
  border-color: var(--color-border-hover);
  color: var(--color-text-primary);
  background: var(--color-surface-glass);
}

/* Sizes */

.btn--default-size,
.btn--default:not(.btn--sm):not(.btn--lg):not(.btn--icon) {
  height: 40px;
  padding: 0 20px;
}

.btn--sm {
  height: 32px;
  padding: 0 12px;
  font-size: 12px;
}

.btn--lg {
  height: 48px;
  padding: 0 24px;
  font-size: 16px;
}

.btn--icon {
  height: 36px;
  width: 36px;
  padding: 0;
}

/* Utility */
.w-full {
  width: 100%;
}
</style>
