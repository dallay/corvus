<script setup lang="ts">
defineOptions({ inheritAttrs: false });

const props = defineProps<{
  variant?: "primary" | "secondary" | "ghost" | "destructive";
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
      `btn--${props.variant ?? 'primary'}`,
      `btn-size--${props.size ?? 'default'}`,
      ($attrs.class as string),
    ]"
  >
    <slot />
  </button>
</template>

<style scoped>
/* ── Nothing Design System — Button ── */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--corvus-spacing-sm);
  border-radius: var(--corvus-radius-pill);
  font-family: var(--corvus-typography-font-mono);
  font-size: 13px;
  font-weight: 400;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  border: none;
  cursor: pointer;
  transition: all var(--corvus-motion-duration-micro) var(--corvus-motion-easing-default);
  user-select: none;
  outline: none;
  min-height: 44px;
  padding: 12px 24px;
}

.btn:disabled {
  opacity: 0.4;
  pointer-events: none;
  cursor: not-allowed;
}

.btn:focus-visible {
  outline: 2px solid var(--corvus-color-text-primary);
  outline-offset: 2px;
}

/* ── Variants ── */

/* Primary: inverted — white bg / black text (dark), black bg / light text (light) */
.btn--primary {
  background: var(--corvus-color-text-display);
  color: var(--corvus-color-bg-base);
}

.btn--primary:hover:not(:disabled) {
  background: var(--corvus-color-text-primary);
}

.btn--primary:active:not(:disabled) {
  background: var(--corvus-color-text-secondary);
}

/* Secondary: transparent, visible border */
.btn--secondary {
  background: transparent;
  border: 1px solid var(--corvus-color-border-visible);
  color: var(--corvus-color-text-primary);
}

.btn--secondary:hover:not(:disabled) {
  border-color: var(--corvus-color-text-secondary);
  color: var(--corvus-color-text-display);
}

.btn--secondary:active:not(:disabled) {
  background: var(--corvus-color-bg-raised);
}

/* Ghost: no border, no background */
.btn--ghost {
  background: transparent;
  border-radius: 0;
  color: var(--corvus-color-text-secondary);
}

.btn--ghost:hover:not(:disabled) {
  color: var(--corvus-color-text-primary);
}

/* Destructive: accent red border */
.btn--destructive {
  background: transparent;
  border: 1px solid var(--corvus-color-accent-default);
  color: var(--corvus-color-accent-default);
}

.btn--destructive:hover:not(:disabled) {
  background: var(--corvus-color-accent-subtle);
}

/* ── Sizes ── */
.btn-size--default {
  height: 44px;
  padding: 12px 24px;
}

.btn-size--sm {
  height: 36px;
  padding: 8px 16px;
  font-size: 11px;
}

.btn-size--lg {
  height: 52px;
  padding: 14px 32px;
  font-size: 14px;
}

.btn-size--icon {
  height: 44px;
  width: 44px;
  min-width: 44px;
  padding: 0;
}
</style>
