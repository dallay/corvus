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

const WEB_SEARCH_BRAVE_API_KEY_MODES = new Set<AdminConfigForm["web_search_brave_api_key_mode"]>([
  "unchanged",
  "replace",
  "clear",
]);

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function updateField<Key extends keyof AdminConfigForm>(
  key: Key,
  value: AdminConfigForm[Key]
): void {
  let clamped = value;
  if (key === "web_search_max_results") {
    const n = Number(value);
    if (!Number.isFinite(n)) return;
    clamped = `${Math.max(1, Math.min(10, Math.round(n)))}` as AdminConfigForm[Key];
  }
  if (key === "web_search_timeout_secs") {
    const n = Number(value);
    if (!Number.isFinite(n)) return;
    clamped = `${Math.max(1, Math.round(n))}` as AdminConfigForm[Key];
  }
  emit("update:modelValue", {
    ...props.modelValue,
    [key]: clamped,
  });
}

function updateSecretMode(mode: AdminConfigForm["web_search_brave_api_key_mode"]): void {
  const patch: Partial<AdminConfigForm> = { web_search_brave_api_key_mode: mode };
  if (mode !== "replace") {
    patch.web_search_brave_api_key_value = "";
  }
  emit("update:modelValue", { ...props.modelValue, ...patch });
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function handleSecretModeChange(event: Event): void {
  const value = (event.target as HTMLSelectElement).value;
  if (
    WEB_SEARCH_BRAVE_API_KEY_MODES.has(value as AdminConfigForm["web_search_brave_api_key_mode"])
  ) {
    updateSecretMode(value as AdminConfigForm["web_search_brave_api_key_mode"]);
  }
}
</script>

<template>
  <section class="card">
    <h2>{{ $t("sections.webSearch") }}</h2>
    <div class="grid">
      <label class="switch-row">
        <input
          :checked="modelValue.web_search_enabled"
          :disabled="disabled || saving"
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
          :disabled="disabled || saving"
          data-testid="web_search_provider"
          @update:model-value="updateField('web_search_provider', $event)"
        />
      </label>
      <label>
        <span>{{ $t("webSearch.maxResults") }}</span>
        <Input
          :model-value="modelValue.web_search_max_results"
          :disabled="disabled || saving"
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
          :disabled="disabled || saving"
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
          :disabled="disabled || saving"
          class="select-input"
          data-testid="web_search_brave_api_key_mode"
          @change="handleSecretModeChange"
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
          aria-describedby="web-search-brave-api-key-help"
          autocapitalize="off"
          :disabled="disabled || saving"
          inputmode="text"
          spellcheck="false"
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
    <p id="web-search-brave-api-key-help" class="helper">
      API keys support paste from password managers or secure vault tools.
    </p>
    <div class="actions">
      <Button :disabled="disabled || saving" data-testid="save" @click="emit('save')">{{
        $t("form.save")
      }}</Button>
    </div>
  </section>
</template>
