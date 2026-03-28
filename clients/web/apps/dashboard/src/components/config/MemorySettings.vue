<script setup lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
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

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function updateSecretMode(mode: AdminConfigForm["memory_cerebro_auth_token_mode"]): void {
  const patch: Partial<AdminConfigForm> = { memory_cerebro_auth_token_mode: mode };
  if (mode !== "replace") {
    patch.memory_cerebro_auth_token_value = "";
  }
  emit("update:modelValue", { ...props.modelValue, ...patch });
}
</script>

<template>
  <section class="card">
    <h2>{{ $t("sections.memory") }}</h2>
    <div class="grid">
      <label>
        <span>{{ $t("memory.cerebroEndpoint") }}</span>
        <Input
          :model-value="modelValue.memory_cerebro_endpoint"
          data-testid="memory_cerebro_endpoint"
          @update:model-value="updateField('memory_cerebro_endpoint', $event)"
        />
      </label>
      <label>
        <span>{{ $t("memory.cerebroTimeoutMs") }}</span>
        <Input
          :model-value="modelValue.memory_cerebro_timeout_ms"
          type="number"
          min="100"
          data-testid="memory_cerebro_timeout_ms"
          @update:model-value="updateField('memory_cerebro_timeout_ms', $event)"
        />
      </label>
      <label class="switch-row">
        <input
          :checked="modelValue.memory_cerebro_allow_insecure_loopback"
          type="checkbox"
          data-testid="memory_cerebro_allow_insecure_loopback"
          @change="
            updateField(
              'memory_cerebro_allow_insecure_loopback',
              ($event.target as HTMLInputElement).checked
            )
          "
        />
        <span>{{ $t("memory.cerebroAllowInsecureLoopback") }}</span>
      </label>
      <label>
        <span>{{ $t("memory.cerebroAuthTokenMode") }}</span>
        <select
          :value="modelValue.memory_cerebro_auth_token_mode"
          class="select-input"
          data-testid="memory_cerebro_auth_token_mode"
          @change="
            updateSecretMode(
              ($event.target as HTMLSelectElement).value as AdminConfigForm['memory_cerebro_auth_token_mode']
            )
          "
        >
          <option value="unchanged">{{ $t("form.secretUnchanged") }}</option>
          <option value="replace">{{ $t("form.secretReplace") }}</option>
          <option value="clear">{{ $t("form.secretClear") }}</option>
        </select>
      </label>
      <label v-if="modelValue.memory_cerebro_auth_token_mode === 'replace'">
        <span>{{ $t("memory.cerebroAuthTokenValue") }}</span>
        <Input
          :model-value="modelValue.memory_cerebro_auth_token_value"
          type="password"
          data-testid="memory_cerebro_auth_token_value"
          @update:model-value="updateField('memory_cerebro_auth_token_value', $event)"
        />
      </label>
    </div>
    <p class="helper">
      {{
        $t("webhook.secretStatus", {
          status: modelValue.memory_cerebro_has_auth_token
            ? $t("webhook.statusConfigured")
            : $t("webhook.statusNotConfigured"),
        })
      }}
    </p>
    <div class="actions">
      <Button :disabled="disabled || saving" data-testid="save" @click="emit('save')">{{
        $t("form.save")
      }}</Button>
    </div>
  </section>
</template>
