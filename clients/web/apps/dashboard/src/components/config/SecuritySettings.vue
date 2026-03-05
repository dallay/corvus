<script setup lang="ts">
import type { AdminConfigForm } from "@/types/admin-config";

const props = defineProps<{
  modelValue: AdminConfigForm;
  autonomyLevelOptions: string[];
  disabled: boolean;
  saving: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: AdminConfigForm];
  save: [];
}>();

function _updateField<Key extends keyof AdminConfigForm>(
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
    <h2>{{ $t("sections.autonomy") }}</h2>
    <div class="grid">
      <label>
        <span>{{ $t("form.autonomyLevel") }}</span>
        <select
          :value="modelValue.autonomy_level"
          class="select-input"
          @change="updateField('autonomy_level', ($event.target as HTMLSelectElement).value)"
        >
          <option v-for="level in autonomyLevelOptions" :key="level" :value="level">
            {{ level }}
          </option>
        </select>
      </label>
      <label>
        <span>{{ $t("form.maxActionsPerHour") }}</span>
        <Input
          :model-value="modelValue.autonomy_max_actions_per_hour"
          type="number"
          min="0"
          @update:model-value="updateField('autonomy_max_actions_per_hour', $event)"
        />
      </label>
      <label>
        <span>{{ $t("form.maxCostPerDayCents") }}</span>
        <Input
          :model-value="modelValue.autonomy_max_cost_per_day_cents"
          type="number"
          min="0"
          @update:model-value="updateField('autonomy_max_cost_per_day_cents', $event)"
        />
      </label>
      <label>
        <span>Identity format</span>
        <Input :model-value="modelValue.identity_format" @update:model-value="updateField('identity_format', $event)" />
      </label>
      <label>
        <span>Identity AIEOS path</span>
        <Input
          :model-value="modelValue.identity_aieos_path"
          @update:model-value="updateField('identity_aieos_path', $event)"
        />
      </label>
      <label class="switch-row">
        <input
          :checked="modelValue.autonomy_workspace_only"
          type="checkbox"
          @change="updateField('autonomy_workspace_only', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t("form.workspaceOnly") }}</span>
      </label>
    </div>
    <div class="actions">
      <Button :disabled="disabled || saving" @click="emit('save')">{{ $t("form.save") }}</Button>
    </div>
  </section>
</template>
