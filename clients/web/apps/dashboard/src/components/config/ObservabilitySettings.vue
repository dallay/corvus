<script setup lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import { Button, Input } from "@corvus/ui";
import type { AdminConfigForm } from "@/types/admin-config";

const props = defineProps<{
  modelValue: AdminConfigForm;
  observabilityBackendOptions: string[];
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
    <h2>{{ $t("sections.observability") }}</h2>
    <div class="grid">
      <label>
        <span>{{ $t("form.observabilityBackend") }}</span>
        <select
          :value="modelValue.observability_backend"
          class="select-input"
          @change="updateField('observability_backend', ($event.target as HTMLSelectElement).value)"
        >
          <option v-for="backend in observabilityBackendOptions" :key="backend" :value="backend">
            {{ backend }}
          </option>
        </select>
      </label>
      <label>
        <span>{{ $t("form.otelEndpoint") }}</span>
        <Input :model-value="modelValue.otel_endpoint" @update:model-value="updateField('otel_endpoint', $event)" />
      </label>
      <label>
        <span>{{ $t("form.otelServiceName") }}</span>
        <Input
          :model-value="modelValue.otel_service_name"
          @update:model-value="updateField('otel_service_name', $event)"
        />
      </label>
    </div>
    <div class="actions">
      <Button :disabled="disabled || saving" @click="emit('save')">{{ $t("form.save") }}</Button>
    </div>
  </section>
</template>
