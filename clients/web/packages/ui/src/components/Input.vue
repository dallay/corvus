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
.form-input {
  display: flex;
  height: 42px;
  width: 100%;
  border-radius: 12px;
  border: 1px solid var(--color-border);
  background: var(--color-bg-input);
  padding: 0 14px;
  font-size: 14px;
  font-family: inherit;
  color: var(--color-text-primary);
  transition: all 0.2s;
  outline: none;
}

.form-input::placeholder {
  color: var(--color-text-muted);
}

.form-input:focus {
  border-color: var(--color-border-accent);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.15); /* Tailwind emerald fallback, gets overridden when css var exists */
  background: var(--color-bg-secondary);
}

@supports (box-shadow: 0 0 0 3px var(--color-accent-glow)) {
  .form-input:focus {
    box-shadow: 0 0 0 3px var(--color-accent-glow);
  }
}

.form-input:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
