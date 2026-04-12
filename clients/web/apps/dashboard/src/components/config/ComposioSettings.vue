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
function updateSecretMode(mode: AdminConfigForm["composio_api_key_mode"]): void {
  const patch: Partial<AdminConfigForm> = { composio_api_key_mode: mode };
  if (mode !== "replace") {
    patch.composio_api_key_value = "";
  }
  emit("update:modelValue", { ...props.modelValue, ...patch });
}
</script>

<template>
  <section class="card">
    <h2>{{ $t("sections.composio") }}</h2>
    <div class="grid">
      <label class="switch-row">
        <input
          :checked="modelValue.composio_enabled"
          type="checkbox"
          data-testid="composio_enabled"
          @change="
            updateField('composio_enabled', ($event.target as HTMLInputElement).checked)
          "
        />
        <span>{{ $t("composio.enabled") }}</span>
      </label>
      <label>
        <span>{{ $t("composio.entityId") }}</span>
        <Input
          :model-value="modelValue.composio_entity_id"
          data-testid="composio_entity_id"
          @update:model-value="updateField('composio_entity_id', $event)"
        />
      </label>
      <label>
        <span>{{ $t("composio.apiKeyMode") }}</span>
        <select
          :value="modelValue.composio_api_key_mode"
          class="select-input"
          data-testid="composio_api_key_mode"
          @change="
            updateSecretMode(
              ($event.target as HTMLSelectElement).value as AdminConfigForm['composio_api_key_mode']
            )
          "
        >
          <option value="unchanged">{{ $t("form.secretUnchanged") }}</option>
          <option value="replace">{{ $t("form.secretReplace") }}</option>
          <option value="clear">{{ $t("form.secretClear") }}</option>
        </select>
      </label>
      <label v-if="modelValue.composio_api_key_mode === 'replace'">
        <span>{{ $t("composio.apiKeyValue") }}</span>
        <Input
          :model-value="modelValue.composio_api_key_value"
          aria-describedby="composio-api-key-help"
          autocapitalize="off"
          inputmode="text"
          spellcheck="false"
          type="password"
          data-testid="composio_api_key_value"
          @update:model-value="updateField('composio_api_key_value', $event)"
        />
      </label>
    </div>
    <p class="helper">
      {{
        $t("webhook.secretStatus", {
          status: modelValue.composio_has_api_key
            ? $t("webhook.statusConfigured")
            : $t("webhook.statusNotConfigured"),
        })
      }}
    </p>
    <p id="composio-api-key-help" class="helper">
      API keys support paste from password managers or secure vault tools.
    </p>
    <div class="actions">
      <Button :disabled="disabled || saving" data-testid="save" @click="emit('save')">{{
        $t("form.save")
      }}</Button>
    </div>
  </section>
</template>
