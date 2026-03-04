<script setup lang="ts">
import type { AdminConfigForm } from "@/types/admin-config";

import Button from "@/components/ui/button/Button.vue";

const props = defineProps<{
  modelValue: AdminConfigForm;
  runtimeKindOptions: string[];
  disabled: boolean;
  saving: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: AdminConfigForm];
  save: [];
}>();

function updateRuntimeKind(kind: string): void {
  emit("update:modelValue", {
    ...props.modelValue,
    runtime_kind: kind,
  });
}
</script>

<template>
  <section class="card">
    <h2>{{ $t("sections.runtime") }}</h2>
    <div class="grid">
      <label>
        <span>{{ $t("form.runtimeKind") }}</span>
        <select
          :value="modelValue.runtime_kind"
          class="select-input"
          @change="updateRuntimeKind(($event.target as HTMLSelectElement).value)"
        >
          <option v-for="kind in runtimeKindOptions" :key="kind" :value="kind">
            {{ kind }}
          </option>
        </select>
      </label>
    </div>
    <div class="actions">
      <Button :disabled="disabled || saving" @click="emit('save')">{{ $t("form.save") }}</Button>
    </div>
  </section>
</template>
