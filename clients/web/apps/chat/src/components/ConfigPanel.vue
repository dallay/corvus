<script setup lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import { Button, Input } from "@corvus/ui";
import { useI18n } from "vue-i18n";

import type { WebChatOnboardingState, WebChatOnboardingStep } from "@/composables/useGateway";

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const props = defineProps<{
  baseUrl: string;
  pairingCode: string;
  bearerToken: string;
  webhookSecret: string;
  loading: boolean;
  statusMessage: string;
  errorMessage: string;
  onboardingState: WebChatOnboardingState;
  onboardingSteps: WebChatOnboardingStep[];
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const emit = defineEmits<{
  "update:baseUrl": [value: string];
  "update:pairingCode": [value: string];
  "update:bearerToken": [value: string];
  "update:webhookSecret": [value: string];
  pair: [];
  connect: [];
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();
</script>

<template>
  <section class="config-card">
    <div class="config-header">
      <div>
        <p class="eyebrow">{{ t("sections.auth") }}</p>
        <h2>{{ t("app.gatewayConfig") }}</h2>
        <p class="config-copy">{{ t("chatOnboarding.intro") }}</p>
      </div>
    </div>

    <ol class="onboarding-steps" aria-label="Web chat onboarding steps">
      <li
        v-for="step in props.onboardingSteps"
        :key="step.key"
        class="onboarding-step"
        :data-step-status="step.status"
      >
        <div class="step-copy">
          <h3>{{ t(step.titleKey) }}</h3>
          <p>{{ t(step.descriptionKey) }}</p>
        </div>
        <span class="step-badge">{{ t(`onboarding.stepStatus.${step.status}`) }}</span>
      </li>
    </ol>

    <div
      v-if="props.onboardingState.state === 'blocked' && props.onboardingState.recoveryKind"
      class="banner banner-error"
      role="alert"
      aria-live="assertive"
    >
      <p class="banner-title">
        {{ t(`chatOnboarding.recovery.${props.onboardingState.recoveryKind}.title`) }}
      </p>
      <p>{{ t(`chatOnboarding.recovery.${props.onboardingState.recoveryKind}.description`) }}</p>
    </div>

    <div v-else-if="props.onboardingState.state === 'ready'" class="banner banner-success">
      <p class="banner-title">{{ t("chatOnboarding.ready.title") }}</p>
      <p>{{ t("chatOnboarding.ready.description") }}</p>
    </div>

    <div class="field-grid">
      <label class="field">
        <span>{{ t("auth.baseUrl") }}</span>
        <Input
          :model-value="props.baseUrl"
          :placeholder="t('form.baseUrlPlaceholder')"
          @update:model-value="(value: string) => emit('update:baseUrl', value)"
        />
      </label>

      <label class="field">
        <span>{{ t("auth.pairingCode") }}</span>
        <Input
          :model-value="props.pairingCode"
          :placeholder="t('form.pairingCodePlaceholder')"
          type="password"
          @update:model-value="(value: string) => emit('update:pairingCode', value)"
        />
      </label>

      <label class="field">
        <span>{{ t("auth.bearerToken") }}</span>
        <Input
          :model-value="props.bearerToken"
          :placeholder="t('form.bearerTokenPlaceholder')"
          type="password"
          @update:model-value="(value: string) => emit('update:bearerToken', value)"
        />
      </label>

      <label class="field">
        <span>{{ t("form.webhookSecret") }}</span>
        <Input
          :model-value="props.webhookSecret"
          :placeholder="t('form.webhookSecretPlaceholder')"
          type="password"
          @update:model-value="(value: string) => emit('update:webhookSecret', value)"
        />
      </label>
    </div>

    <div class="config-actions">
      <Button :disabled="props.loading" @click="emit('pair')">{{ t("auth.pair") }}</Button>
      <Button :disabled="props.loading" variant="secondary" @click="emit('connect')">
        {{ t("auth.connect") }}
      </Button>
    </div>

    <p v-if="props.statusMessage" class="status status-ok">{{ props.statusMessage }}</p>
    <p v-if="props.errorMessage" class="status status-error">{{ props.errorMessage }}</p>
  </section>
</template>

<style scoped>
.config-card {
  width: 100%;
  max-width: 720px;
  display: grid;
  gap: 16px;
  border-radius: 20px;
  border: 1px solid var(--corvus-color-border-default);
  background: var(--corvus-color-bg-surface);
  padding: 24px;
}

.config-header h2,
.step-copy h3 {
  margin: 0;
}

.eyebrow {
  margin: 0 0 6px;
  font-size: 12px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--corvus-color-text-disabled);
}

.config-copy,
.step-copy p,
.banner p,
.status {
  margin: 6px 0 0;
}

.onboarding-steps {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: 10px;
}

.onboarding-step {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  border: 1px solid var(--corvus-color-border-default);
  border-radius: var(--corvus-radius-card);
  padding: 14px;
  background: var(--corvus-color-bg-surface);
}

.onboarding-step[data-step-status="complete"] {
  border-color: var(--corvus-color-status-success);
}

.onboarding-step[data-step-status="current"] {
  border-color: var(--corvus-color-interactive-default);
}

.onboarding-step[data-step-status="blocked"] {
  border-color: var(--corvus-color-status-error);
}

.step-badge {
  flex-shrink: 0;
  align-self: flex-start;
  border-radius: var(--corvus-radius-pill);
  background: var(--corvus-color-bg-raised);
  color: var(--corvus-color-text-secondary);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  padding: 4px 10px;
  text-transform: uppercase;
}

.banner {
  border-radius: var(--corvus-radius-card);
  padding: 14px;
}

.banner-title {
  font-weight: 600;
}

.banner-success {
  border: 1px solid var(--corvus-color-status-success);
  background: var(--corvus-color-bg-surface);
}

.banner-error {
  border: 1px solid var(--corvus-color-status-error);
  background: var(--corvus-color-bg-surface);
}

.field-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
}

.field {
  display: grid;
  gap: 6px;
}

.field span {
  font-size: 13px;
  color: var(--corvus-color-text-secondary);
}

.config-actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.status-ok {
  color: var(--corvus-color-status-success);
}

.status-error {
  color: var(--corvus-color-status-error);
}

@media (max-width: 767px) {
  .config-card {
    padding: 18px;
  }

  .onboarding-step {
    flex-direction: column;
  }
}
</style>
