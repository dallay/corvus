<script setup lang="ts">
defineOptions({ inheritAttrs: false });

defineProps<{
  modelValue?: string;
  placeholder?: string;
  type?: string;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
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
  border-radius: 10px;
  border: 1px solid var(--color-border);
  background: var(--color-bg-input);
  padding: 0 12px;
  font-size: 14px;
  font-family: inherit;
  color: var(--color-text-primary);
  outline: none;
}

.form-input:focus {
  border-color: var(--color-border-accent);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.15);
}

.form-input::placeholder {
  color: var(--color-text-muted);
}
</style>
