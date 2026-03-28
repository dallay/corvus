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
function updateSecretMode(mode: AdminConfigForm["web_search_brave_api_key_mode"]): void {
  const patch: Partial<AdminConfigForm> = { web_search_brave_api_key_mode: mode };
  if (mode !== "replace") {
    patch.web_search_brave_api_key_value = "";
  }
  emit("update:modelValue", { ...props.modelValue, ...patch });
}
</script>

<template>
  <section class="card">
    <h2>{{ $t("sections.webSearch") }}</h2>
    <div class="grid">
      <label class="switch-row">
        <input
          :checked="modelValue.web_search_enabled"
          type="checkbox"
          data-testid="web_search_enabled"
          @change="
            updateField('web_search_enabled', ($event.target as HTMLInputElement).checked)
          "
        />
        <span>{{ $t("webSearch.enabled") }}</span>
      </label>
      <label>
        <span>{{ $t("webSearch.provider") }}</span>
        <Input
          :model-value="modelValue.web_search_provider"
          data-testid="web_search_provider"
          @update:model-value="updateField('web_search_provider', $event)"
        />
      </label>
      <label>
        <span>{{ $t("webSearch.maxResults") }}</span>
        <Input
          :model-value="modelValue.web_search_max_results"
          type="number"
          min="1"
          max="10"
          data-testid="web_search_max_results"
          @update:model-value="updateField('web_search_max_results', $event)"
        />
      </label>
      <label>
        <span>{{ $t("webSearch.timeoutSecs") }}</span>
        <Input
          :model-value="modelValue.web_search_timeout_secs"
          type="number"
          min="1"
          data-testid="web_search_timeout_secs"
          @update:model-value="updateField('web_search_timeout_secs', $event)"
        />
      </label>
      <label>
        <span>{{ $t("webSearch.braveApiKeyMode") }}</span>
        <select
          :value="modelValue.web_search_brave_api_key_mode"
          class="select-input"
          data-testid="web_search_brave_api_key_mode"
          @change="
            updateSecretMode(
              ($event.target as HTMLSelectElement).value as AdminConfigForm['web_search_brave_api_key_mode']
            )
          "
        >
          <option value="unchanged">{{ $t("form.secretUnchanged") }}</option>
          <option value="replace">{{ $t("form.secretReplace") }}</option>
          <option value="clear">{{ $t("form.secretClear") }}</option>
        </select>
      </label>
      <label v-if="modelValue.web_search_brave_api_key_mode === 'replace'">
        <span>{{ $t("webSearch.braveApiKeyValue") }}</span>
        <Input
          :model-value="modelValue.web_search_brave_api_key_value"
          type="password"
          data-testid="web_search_brave_api_key_value"
          @update:model-value="updateField('web_search_brave_api_key_value', $event)"
        />
      </label>
    </div>
    <p class="helper">
      {{
        $t("webhook.secretStatus", {
          status: modelValue.web_search_has_brave_api_key
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
