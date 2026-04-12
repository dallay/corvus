<script setup lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import { Button, Input } from "@corvus/ui";
import { computed, nextTick, ref } from "vue";
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
const errorSummaryRef = ref<HTMLElement | null>(null);
const webhookSecretInputRef = ref<HTMLInputElement | null>(null);

const localError = computed(() => {
  if (
    props.modelValue.webhook_secret_mode === "replace" &&
    !props.modelValue.webhook_secret_value.trim()
  ) {
    return "secret-required";
  }
  return "";
});

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
function handleSave(): void {
  if (localError.value) {
    globalThis.requestAnimationFrame(() => {
      errorSummaryRef.value?.focus();
    });
    return;
  }
  emit("save");
}

async function _focusWebhookSecretField(): Promise<void> {
  await nextTick();
  webhookSecretInputRef.value?.focus();
}
</script>

<template>
  <section class="card">
    <h2>{{ $t("sections.webhook") }}</h2>
    <div
      v-if="localError"
      ref="errorSummaryRef"
      aria-labelledby="webhook-error-summary-title"
      class="error-summary"
      role="alert"
      tabindex="-1"
    >
      <p id="webhook-error-summary-title" class="error-summary-title">
        Please fix the following error before saving.
      </p>
      <button class="error-summary-link" type="button" @click="focusWebhookSecretField">
        {{ $t("auth.emptyWebhookSecret") }}
      </button>
    </div>
    <div class="grid">
      <label class="switch-row">
        <input
          :checked="modelValue.webhook_enabled"
          type="checkbox"
          @change="updateField('webhook_enabled', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t("webhook.enabled") }}</span>
      </label>
      <label>
        <span>{{ $t("form.webhookPort") }}</span>
        <Input
          :model-value="modelValue.webhook_port"
          type="number"
          min="1"
          max="65535"
          @update:model-value="updateField('webhook_port', $event)"
        />
      </label>
      <label>
        <span>{{ $t("form.webhookSecretMode") }}</span>
        <select
          :value="modelValue.webhook_secret_mode"
          class="select-input"
          @change="updateField('webhook_secret_mode', ($event.target as HTMLSelectElement).value as AdminConfigForm['webhook_secret_mode'])"
        >
          <option value="unchanged">{{ $t("form.secretUnchanged") }}</option>
          <option value="replace">{{ $t("form.secretReplace") }}</option>
          <option value="clear">{{ $t("form.secretClear") }}</option>
        </select>
      </label>
      <label v-if="modelValue.webhook_secret_mode === 'replace'">
        <span>{{ $t("form.webhookSecretValue") }}</span>
        <Input
          ref="webhookSecretInputRef"
          :model-value="modelValue.webhook_secret_value"
          :aria-describedby="localError ? 'webhook-secret-help webhook-secret-error' : 'webhook-secret-help'"
          :aria-invalid="localError ? 'true' : 'false'"
          autocapitalize="off"
          inputmode="text"
          spellcheck="false"
          type="password"
          @update:model-value="updateField('webhook_secret_value', $event)"
        />
      </label>
    </div>
    <p class="helper">
      {{ $t("webhook.secretStatus", { status: modelValue.webhook_secret_exists ? $t("webhook.statusConfigured") : $t("webhook.statusNotConfigured") }) }}
    </p>
    <p id="webhook-secret-help" class="helper">
      Webhook secrets support paste from password managers or secure vault tools.
    </p>
    <p v-if="localError" id="webhook-secret-error" class="error">{{ $t("auth.emptyWebhookSecret") }}</p>
    <div class="actions">
      <Button :disabled="disabled || saving || !!localError" @click="handleSave">{{ $t("form.save") }}</Button>
    </div>
  </section>
</template>

<style scoped>
.error-summary {
  border: 1px solid var(--corvus-color-status-error);
  border-radius: var(--corvus-radius-input);
  display: grid;
  gap: 8px;
  margin-bottom: 16px;
  padding: 12px 16px;
}

.error-summary-title {
  color: var(--corvus-color-status-error);
  font-weight: 600;
  margin: 0;
}

.error-summary-link {
  background: none;
  border: 0;
  color: var(--corvus-color-status-error);
  cursor: pointer;
  justify-self: start;
  padding: 0;
  text-align: left;
  text-decoration: underline;
}
</style>
