<script setup lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import { Button, Input } from "@corvus/ui";
import { useI18n } from "vue-i18n";

// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import GatewaySettings from "@/components/config/GatewaySettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import GeneralSettings from "@/components/config/GeneralSettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import ObservabilitySettings from "@/components/config/ObservabilitySettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import RuntimeSettings from "@/components/config/RuntimeSettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import SchedulerSettings from "@/components/config/SchedulerSettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import SecuritySettings from "@/components/config/SecuritySettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import WebhookSettings from "@/components/config/WebhookSettings.vue";
import { useConfig } from "@/composables/useConfig";

const { t } = useI18n();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const config = useConfig(t);
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
      <div v-if="config.quickPairState.value === 'validating' || config.quickPairState.value === 'pairing'" class="quick-pair-state">
        <p>{{ t("auth.quickPairValidating") }}</p>
      </div>
      <div v-else-if="config.quickPairState.value === 'connecting'" class="quick-pair-state">
        <p>{{ t("auth.quickPairConnecting") }}</p>
      </div>
      <div v-else>
        <p v-if="config.quickPairState.value === 'failed'" class="error">{{ t("auth.quickPairFailed") }}</p>
        <div class="grid">
          <label>
            <span>{{ t("auth.baseUrl") }}</span>
            <Input v-model="config.baseUrl.value" :placeholder="t('form.baseUrlPlaceholder')" />
          </label>
          <label>
            <span>{{ t("auth.pairingCode") }}</span>
            <Input v-model="config.pairingCode.value" type="password" />
          </label>
          <label>
            <span>{{ t("auth.bearerToken") }}</span>
            <Input v-model="config.bearerToken.value" type="password" />
          </label>
        </div>
        <div class="actions">
          <Button :disabled="config.loading.value" @click="config.pairGateway">{{ t("auth.pair") }}</Button>
          <Button :disabled="config.loading.value" variant="outline" @click="config.connectGateway">
            {{ t("auth.connect") }}
          </Button>
        </div>
      </div>
    </section>

    <GeneralSettings
      :model-value="config.form"
      :memory-backend-options="config.memoryBackendOptions.value"
      :disabled="!config.canSave.value"
      :saving="config.sectionSaving.general"
      @update:model-value="Object.assign(config.form, $event)"
      @save="config.saveSection('general')"
    />

    <SecuritySettings
      :model-value="config.form"
      :autonomy-level-options="config.autonomyLevelOptions.value"
      :disabled="!config.canSave.value"
      :saving="config.sectionSaving.security"
      @update:model-value="Object.assign(config.form, $event)"
      @save="config.saveSection('security')"
    />

    <ObservabilitySettings
      :model-value="config.form"
      :observability-backend-options="config.observabilityBackendOptions.value"
      :disabled="!config.canSave.value"
      :saving="config.sectionSaving.observability"
      @update:model-value="Object.assign(config.form, $event)"
      @save="config.saveSection('observability')"
    />

    <RuntimeSettings
      :model-value="config.form"
      :runtime-kind-options="config.runtimeKindOptions.value"
      :disabled="!config.canSave.value"
      :saving="config.sectionSaving.runtime"
      @update:model-value="Object.assign(config.form, $event)"
      @save="config.saveSection('runtime')"
    />

    <SchedulerSettings
      :model-value="config.form"
      :disabled="!config.canSave.value"
      :saving="config.sectionSaving.scheduler"
      @update:model-value="Object.assign(config.form, $event)"
      @save="config.saveSection('scheduler')"
    />

    <GatewaySettings
      :model-value="config.form"
      :disabled="!config.canSave.value"
      :saving="config.sectionSaving.gateway"
      @update:model-value="Object.assign(config.form, $event)"
      @save="config.saveSection('gateway')"
    />

    <WebhookSettings
      :model-value="config.form"
      :disabled="!config.canSave.value"
      :saving="config.sectionSaving.webhook"
      @update:model-value="Object.assign(config.form, $event)"
      @save="config.saveSection('webhook')"
    />

    <section class="card">
      <p class="helper">
        {{
          t("webhook.secretStatus", {
            status: config.form.webhook_secret_exists
              ? t("webhook.statusConfigured")
              : t("webhook.statusNotConfigured"),
          })
        }}
      </p>
      <p v-if="config.statusMessage.value" class="ok">{{ config.statusMessage.value }}</p>
      <p v-if="config.errorMessage.value" class="error">{{ config.errorMessage.value }}</p>
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

.quick-pair-state p {
  margin: 10px 0;
  font-size: 14px;
  color: var(--color-text-secondary);
}
</style>
