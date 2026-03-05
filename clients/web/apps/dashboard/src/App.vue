<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import GatewaySettings from "@/components/config/GatewaySettings.vue";
import GeneralSettings from "@/components/config/GeneralSettings.vue";
import ObservabilitySettings from "@/components/config/ObservabilitySettings.vue";
import RuntimeSettings from "@/components/config/RuntimeSettings.vue";
import SchedulerSettings from "@/components/config/SchedulerSettings.vue";
import SecuritySettings from "@/components/config/SecuritySettings.vue";
import WebhookSettings from "@/components/config/WebhookSettings.vue";
import Button from "@/components/ui/button/Button.vue";
import Input from "@/components/ui/input/Input.vue";
import { useConfig } from "@/composables/useConfig";

const { t } = useI18n();

const config = useConfig(t);
const {
  baseUrl,
  pairingCode,
  bearerToken,
  loading,
  statusMessage,
  errorMessage,
  form,
  canSave,
  sectionSaving,
  pairGateway,
  connectGateway,
  saveSection,
} = config;

const webhookSecretStatusLabel = computed(() =>
  form.webhook_secret_exists ? t("webhook.statusConfigured") : t("webhook.statusNotConfigured")
);
</script>

<template>
  <main class="dashboard-shell">
    <header class="header-card">
      <div class="header-title-row">
        <img src="/favicon-light.svg" alt="Corvus" width="32" height="32" class="header-logo" />
        <div>
          <h1>{{ t("app.title") }}</h1>
          <p>{{ t("app.subtitle") }}</p>
        </div>
      </div>
    </header>

    <section class="card">
      <h2>{{ t("sections.auth") }}</h2>
      <div class="grid">
        <label>
          <span>{{ t("auth.baseUrl") }}</span>
          <Input v-model="baseUrl" placeholder="http://127.0.0.1:3000" />
        </label>
        <label>
          <span>{{ t("auth.pairingCode") }}</span>
          <Input v-model="pairingCode" type="password" />
        </label>
        <label>
          <span>{{ t("auth.bearerToken") }}</span>
          <Input v-model="bearerToken" type="password" />
        </label>
      </div>
      <div class="actions">
        <Button :disabled="loading" @click="pairGateway">{{ t("auth.pair") }}</Button>
        <Button :disabled="loading" variant="outline" @click="connectGateway">
          {{ t("auth.connect") }}
        </Button>
      </div>
    </section>

    <GeneralSettings
      :model-value="form"
      :memory-backend-options="config.memoryBackendOptions.value"
      :disabled="!canSave"
      :saving="sectionSaving.general"
      @update:model-value="Object.assign(form, $event)"
      @save="saveSection('general')"
    />

    <SecuritySettings
      :model-value="form"
      :autonomy-level-options="config.autonomyLevelOptions.value"
      :disabled="!canSave"
      :saving="sectionSaving.security"
      @update:model-value="Object.assign(form, $event)"
      @save="saveSection('security')"
    />

    <ObservabilitySettings
      :model-value="form"
      :observability-backend-options="config.observabilityBackendOptions.value"
      :disabled="!canSave"
      :saving="sectionSaving.observability"
      @update:model-value="Object.assign(form, $event)"
      @save="saveSection('observability')"
    />

    <RuntimeSettings
      :model-value="form"
      :runtime-kind-options="config.runtimeKindOptions.value"
      :disabled="!canSave"
      :saving="sectionSaving.runtime"
      @update:model-value="Object.assign(form, $event)"
      @save="saveSection('runtime')"
    />

    <SchedulerSettings
      :model-value="form"
      :disabled="!canSave"
      :saving="sectionSaving.scheduler"
      @update:model-value="Object.assign(form, $event)"
      @save="saveSection('scheduler')"
    />

    <GatewaySettings
      :model-value="form"
      :disabled="!canSave"
      :saving="sectionSaving.gateway"
      @update:model-value="Object.assign(form, $event)"
      @save="saveSection('gateway')"
    />

    <WebhookSettings
      :model-value="form"
      :disabled="!canSave"
      :saving="sectionSaving.webhook"
      @update:model-value="Object.assign(form, $event)"
      @save="saveSection('webhook')"
    />

    <section class="card">
      <p class="helper">{{ t("webhook.secretStatus", { status: webhookSecretStatusLabel }) }}</p>
      <p v-if="statusMessage" class="ok">{{ statusMessage }}</p>
      <p v-if="errorMessage" class="error">{{ errorMessage }}</p>
    </section>
  </main>
</template>

<style scoped>
.dashboard-shell {
  max-width: 1040px;
  margin: 0 auto;
  padding: 24px;
  display: grid;
  gap: 16px;
}

.header-card,
.card {
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: 16px;
  padding: 16px;
}

.header-card h1 {
  margin: 0;
  font-size: 24px;
}

.header-card p {
  margin: 6px 0 0;
  color: var(--color-text-secondary);
}

.header-title-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-logo {
  flex-shrink: 0;
}

h2 {
  margin: 0 0 12px;
  font-size: 16px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 12px;
}

label {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

label span {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.select-input {
  height: 42px;
  border-radius: 10px;
  border: 1px solid var(--color-border);
  background: var(--color-bg-input);
  color: var(--color-text-primary);
  font-family: inherit;
  padding: 0 10px;
}

.actions {
  margin-top: 12px;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.switch-row {
  flex-direction: row;
  align-items: center;
  gap: 8px;
  margin-top: 20px;
}

.helper,
.ok,
.error {
  margin: 10px 0 0;
  font-size: 13px;
}

.helper {
  color: var(--color-text-muted);
}

.ok {
  color: #22c55e;
}

.error {
  color: #ef4444;
}
</style>
