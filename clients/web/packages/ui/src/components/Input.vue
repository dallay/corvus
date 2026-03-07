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
  width: 100%;
  height: 42px;
  padding: 0 14px;
  font-family: inherit;
  font-size: 14px;
  color: var(--color-text-primary);
  outline: none;
  background: var(--color-bg-input);
  border: 1px solid var(--color-border);
  border-radius: 12px;
  transition: all 0.2s;
}

.form-input::placeholder {
  color: var(--color-text-muted);
}

.form-input:focus {
  background: var(--color-bg-secondary);
  border-color: var(--color-border-accent);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.15); /* Tailwind emerald fallback, gets overridden when css var exists */
}

@supports (box-shadow: 0 0 0 3px var(--color-accent-glow)) {
  .form-input:focus {
    box-shadow: 0 0 0 3px var(--color-accent-glow);
  }
}

.form-input:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}
</style>
