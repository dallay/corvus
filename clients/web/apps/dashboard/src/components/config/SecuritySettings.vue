<script setup lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import { Button, Input } from "@corvus/ui";
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
        <span>{{ $t("security.identity_format") }}</span>
        <Input :model-value="modelValue.identity_format" @update:model-value="updateField('identity_format', $event)" />
      </label>
      <label>
        <span>{{ $t("security.identity_aieos_path") }}</span>
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
      <label class="switch-row">
        <input
          data-testid="autonomy-require-approval-medium-risk"
          :checked="modelValue.autonomy_require_approval_for_medium_risk"
          type="checkbox"
          @change="updateField('autonomy_require_approval_for_medium_risk', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t("security.requireApprovalMediumRisk") }}</span>
      </label>
      <label class="switch-row">
        <input
          data-testid="autonomy-block-high-risk-commands"
          :checked="modelValue.autonomy_block_high_risk_commands"
          type="checkbox"
          @change="updateField('autonomy_block_high_risk_commands', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t("security.blockHighRiskCommands") }}</span>
      </label>
      <label>
        <span>{{ $t("security.autoApprove") }}</span>
        <textarea
          data-testid="autonomy-auto-approve"
          :value="modelValue.autonomy_auto_approve"
          placeholder="command1, command2"
          @input="updateField('autonomy_auto_approve', ($event.target as HTMLTextAreaElement).value)"
        />
      </label>
      <label>
        <span>{{ $t("security.alwaysAsk") }}</span>
        <textarea
          data-testid="autonomy-always-ask"
          :value="modelValue.autonomy_always_ask"
          placeholder="command1, command2"
          @input="updateField('autonomy_always_ask', ($event.target as HTMLTextAreaElement).value)"
        />
      </label>
    </div>
    <div class="actions">
      <Button :disabled="disabled || saving" @click="emit('save')">{{ $t("form.save") }}</Button>
    </div>
  </section>
</template>
