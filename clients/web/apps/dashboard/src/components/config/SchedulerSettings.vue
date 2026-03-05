<script setup lang="ts">
import { Button, Input } from "@corvus/ui";
import type { AdminConfigForm } from "@/types/admin-config";

const props = defineProps<{
  modelValue: AdminConfigForm;
  disabled: boolean;
  saving: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: AdminConfigForm];
  save: [];
}>();

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
    <h2>{{ $t("sections.scheduler") }}</h2>
    <div class="grid">
      <label class="switch-row">
        <input
          :checked="modelValue.scheduler_enabled"
          type="checkbox"
          @change="updateField('scheduler_enabled', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t("form.schedulerEnabled") }}</span>
      </label>
      <label>
        <span>{{ $t("form.schedulerMaxTasks") }}</span>
        <Input
          :model-value="modelValue.scheduler_max_tasks"
          type="number"
          min="1"
          @update:model-value="updateField('scheduler_max_tasks', $event)"
        />
      </label>
      <label>
        <span>{{ $t("form.schedulerMaxConcurrent") }}</span>
        <Input
          :model-value="modelValue.scheduler_max_concurrent"
          type="number"
          min="1"
          @update:model-value="updateField('scheduler_max_concurrent', $event)"
        />
      </label>
    </div>
    <div class="actions">
      <Button :disabled="disabled || saving" @click="emit('save')">{{ $t("form.save") }}</Button>
    </div>
  </section>
</template>
