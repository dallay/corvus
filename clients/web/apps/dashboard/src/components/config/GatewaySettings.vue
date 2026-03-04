<script setup lang="ts">
import type { AdminConfigForm } from "@/types/admin-config";

import Button from "@/components/ui/button/Button.vue";
import Input from "@/components/ui/input/Input.vue";

const props = defineProps<{
  modelValue: AdminConfigForm;
  disabled: boolean;
  saving: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: AdminConfigForm];
  save: [];
}>();

function updateField<Key extends keyof AdminConfigForm>(key: Key, value: AdminConfigForm[Key]): void {
  emit("update:modelValue", {
    ...props.modelValue,
    [key]: value,
  });
}
</script>

<template>
  <section class="card">
    <h2>{{ $t("sections.gateway") }}</h2>
    <div class="grid">
      <label>
        <span>{{ $t("form.gatewayPort") }}</span>
        <Input
          :model-value="modelValue.gateway_port"
          type="number"
          min="1"
          max="65535"
          @update:model-value="updateField('gateway_port', $event)"
        />
      </label>
      <label>
        <span>{{ $t("form.gatewayHost") }}</span>
        <Input :model-value="modelValue.gateway_host" @update:model-value="updateField('gateway_host', $event)" />
      </label>
      <label>
        <span>Pair rate limit/min</span>
        <Input
          :model-value="modelValue.gateway_pair_rate_limit_per_minute"
          type="number"
          min="1"
          @update:model-value="updateField('gateway_pair_rate_limit_per_minute', $event)"
        />
      </label>
      <label>
        <span>Webhook rate limit/min</span>
        <Input
          :model-value="modelValue.gateway_webhook_rate_limit_per_minute"
          type="number"
          min="1"
          @update:model-value="updateField('gateway_webhook_rate_limit_per_minute', $event)"
        />
      </label>
      <label class="switch-row">
        <input
          :checked="modelValue.gateway_require_pairing"
          type="checkbox"
          @change="updateField('gateway_require_pairing', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t("form.requirePairing") }}</span>
      </label>
      <label class="switch-row">
        <input
          :checked="modelValue.gateway_allow_public_bind"
          type="checkbox"
          @change="updateField('gateway_allow_public_bind', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t("form.allowPublicBind") }}</span>
      </label>
    </div>
    <div class="actions">
      <Button :disabled="disabled || saving" @click="emit('save')">{{ $t("form.save") }}</Button>
    </div>
  </section>
</template>
