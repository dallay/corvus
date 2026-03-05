<script setup lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import { Button, Input } from "@corvus/ui";
import type { AdminConfigForm } from "@/types/admin-config";

const props = defineProps<{
  modelValue: AdminConfigForm;
  memoryBackendOptions: string[];
  disabled: boolean;
  saving: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: AdminConfigForm];
  save: [];
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function updateField<Key extends keyof AdminConfigForm>(
  key: Key,
  value: AdminConfigForm[Key]
): void {
  emit("update:modelValue", {
    ...props.modelValue,
    [key]: value,
  });
}
</script>

<template>
  <section class="card">
    <h2>{{ $t("sections.core") }}</h2>
    <div class="grid">
      <label>
        <span>{{ $t("form.provider") }}</span>
        <Input :model-value="modelValue.default_provider" @update:model-value="updateField('default_provider', $event)" />
      </label>
      <label>
        <span>{{ $t("form.model") }}</span>
        <Input :model-value="modelValue.default_model" @update:model-value="updateField('default_model', $event)" />
      </label>
      <label>
        <span>{{ $t("general.apiUrl") }}</span>
        <Input :model-value="modelValue.api_url" @update:model-value="updateField('api_url', $event)" />
      </label>
      <label>
        <span>{{ $t("form.temperature") }}</span>
        <Input
          :model-value="modelValue.default_temperature"
          type="number"
          step="0.1"
          min="0"
          max="2"
          @update:model-value="updateField('default_temperature', $event)"
        />
      </label>
      <label>
        <span>{{ $t("form.memoryBackend") }}</span>
        <select
          :value="modelValue.memory_backend"
          class="select-input"
          @change="updateField('memory_backend', ($event.target as HTMLSelectElement).value)"
        >
          <option v-for="backend in memoryBackendOptions" :key="backend" :value="backend">
            {{ backend }}
          </option>
        </select>
      </label>
    </div>
    <div class="actions">
      <Button :disabled="disabled || saving" @click="emit('save')">{{ $t("form.save") }}</Button>
    </div>
  </section>
</template>
