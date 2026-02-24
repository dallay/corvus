<script setup lang="ts">
defineOptions({ inheritAttrs: false });

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const props = defineProps<{
  variant?: "default" | "outline";
  type?: "button" | "submit" | "reset";
}>();
</script>

<template>
  <button
    v-bind="$attrs"
    :type="type ?? 'button'"
    :class="['btn', `btn--${props.variant ?? 'default'}`, ($attrs.class as string)]"
  >
    <slot />
  </button>
</template>

<style scoped>
.btn {
  height: 40px;
  border-radius: 10px;
  border: none;
  padding: 0 14px;
  font-size: 14px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn--default {
  color: #fff;
  background: var(--color-accent);
}

.btn--default:hover:not(:disabled) {
  background: var(--color-accent-hover);
}

.btn--outline {
  color: var(--color-text-primary);
  background: transparent;
  border: 1px solid var(--color-border);
}

.btn--outline:hover:not(:disabled) {
  border-color: var(--color-border-hover);
  background: var(--color-surface-glass);
}
</style>
