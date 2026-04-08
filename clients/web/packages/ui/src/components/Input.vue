<script setup lang="ts">
defineOptions({ inheritAttrs: false });

defineProps<{
  modelValue?: string;
  placeholder?: string;
  type?: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<template>
  <input
    v-bind="$attrs"
    :class="['form-input', ($attrs.class as string)]"
    :type="type ?? 'text'"
    :value="modelValue"
    :placeholder="placeholder"
    @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>

<style scoped>
/* ── Nothing Design System — Input ── */
.form-input {
  display: flex;
  height: 44px;
  width: 100%;
  border-radius: 0;
  border: none;
  border-bottom: 1px solid var(--corvus-color-border-visible);
  background: transparent;
  padding: 0 var(--corvus-spacing-xs);
  font-size: var(--corvus-typography-scale-body-sm-size);
  font-family: var(--corvus-typography-font-mono);
  color: var(--corvus-color-text-primary);
  transition: border-color var(--corvus-motion-duration-micro) var(--corvus-motion-easing-default);
  outline: none;
}

.form-input::placeholder {
  color: var(--corvus-color-text-disabled);
}

.form-input:focus {
  border-bottom-color: var(--corvus-color-text-primary);
}

.form-input[aria-invalid="true"] {
  border-bottom-color: var(--corvus-color-accent-default);
}

.form-input:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
