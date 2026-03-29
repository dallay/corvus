<script setup lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.

import { trimTrailingSlashes } from "@corvus/shared";
import { Button, Input } from "@corvus/ui";
import { ref } from "vue";
import { useI18n } from "vue-i18n";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import BrowserSettings from "@/components/config/BrowserSettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import ChannelsOverview from "@/components/config/ChannelsOverview.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import ComposioSettings from "@/components/config/ComposioSettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import CostOverview from "@/components/config/CostOverview.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import GatewaySettings from "@/components/config/GatewaySettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import GeneralSettings from "@/components/config/GeneralSettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import HealthDashboard from "@/components/config/HealthDashboard.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import HeartbeatOverview from "@/components/config/HeartbeatOverview.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import McpOverview from "@/components/config/McpOverview.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import MemorySettings from "@/components/config/MemorySettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import ObservabilitySettings from "@/components/config/ObservabilitySettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import ReliabilityOverview from "@/components/config/ReliabilityOverview.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import RuntimeSettings from "@/components/config/RuntimeSettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import SchedulerSettings from "@/components/config/SchedulerSettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import SchedulerStatus from "@/components/config/SchedulerStatus.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import SecuritySettings from "@/components/config/SecuritySettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import TunnelOverview from "@/components/config/TunnelOverview.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import WebhookSettings from "@/components/config/WebhookSettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import WebSearchSettings from "@/components/config/WebSearchSettings.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import MemoryFilters from "@/components/memory/MemoryFilters.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import MemoryList from "@/components/memory/MemoryList.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import MemoryStats from "@/components/memory/MemoryStats.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import SessionDetail from "@/components/sessions/SessionDetail.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import SessionFilters from "@/components/sessions/SessionFilters.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import SessionList from "@/components/sessions/SessionList.vue";
import { useConfig } from "@/composables/useConfig";
import type { AdminSessionView } from "@/types/admin-sessions";

const { t } = useI18n();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const config = useConfig(t);

type DashboardPage = "config" | "sessions" | "memory";
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const currentPage = ref<DashboardPage>("config");

// Session view state
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const sessionStatusFilter = ref<"active" | "ended" | undefined>(undefined);
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const sessionSort = ref<"last_activity" | "started_at">("last_activity");
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const selectedSession = ref<AdminSessionView | null>(null);

// Memory view state
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const memoryCategoryFilter = ref<string | undefined>(undefined);
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const memorySessionIdFilter = ref<string | undefined>(undefined);
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const memorySearchFilter = ref<string | undefined>(undefined);

// Gateway URL builder and auth headers for useAdmin composable
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function adminGatewayUrl(path: string): string {
  const base = trimTrailingSlashes(config.baseUrl.value.trim()) || "/api";
  if (base.startsWith("/")) {
    return new URL(`${base}${path}`, globalThis.location.origin).toString();
  }
  const cleanPath = path.startsWith("/") ? path.slice(1) : path;
  return new URL(cleanPath, `${trimTrailingSlashes(base)}/`).toString();
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function adminAuthHeaders(): Record<string, string> {
  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (config.bearerToken.value.trim()) {
    headers.Authorization = `Bearer ${config.bearerToken.value.trim()}`;
  }
  return headers;
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function onSelectSession(session: AdminSessionView) {
  selectedSession.value = session;
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function onViewSessionMemory(sessionId: string) {
  memorySessionIdFilter.value = sessionId;
  currentPage.value = "memory";
  selectedSession.value = null;
}
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
      <nav v-if="config.isOperatorReady.value" class="nav-tabs" role="tablist" aria-label="Dashboard navigation">
        <button
          role="tab"
          class="nav-tab"
          :class="{ 'nav-tab-active': currentPage === 'config' }"
          :aria-selected="currentPage === 'config'"
          @click="currentPage = 'config'"
        >
          {{ t("nav.config", "Config") }}
        </button>
        <button
          role="tab"
          class="nav-tab"
          :class="{ 'nav-tab-active': currentPage === 'sessions' }"
          :aria-selected="currentPage === 'sessions'"
          @click="currentPage = 'sessions'"
        >
          {{ t("nav.sessions", "Sessions") }}
        </button>
        <button
          role="tab"
          class="nav-tab"
          :class="{ 'nav-tab-active': currentPage === 'memory' }"
          :aria-selected="currentPage === 'memory'"
          @click="currentPage = 'memory'"
        >
          {{ t("nav.memory", "Memory") }}
        </button>
      </nav>
    </header>

    <!-- Auth / Onboarding section — always visible -->
    <section class="card">
      <h2>{{ t("sections.auth") }}</h2>
      <p class="helper onboarding-intro">{{ t("onboarding.intro") }}</p>
      <ol class="onboarding-steps" aria-label="Dashboard onboarding steps">
        <li
          v-for="step in config.onboardingSteps.value"
          :key="step.key"
          class="onboarding-step"
          :data-step-status="step.status"
        >
          <div class="onboarding-step-header">
            <div>
              <h3>{{ t(step.titleKey) }}</h3>
              <p>{{ t(step.descriptionKey) }}</p>
            </div>
            <span class="step-badge">{{ t(`onboarding.stepStatus.${step.status}`) }}</span>
          </div>
        </li>
      </ol>
      <output
        v-if="config.isOperatorReady.value"
        class="onboarding-banner onboarding-banner-ready"
        aria-live="polite"
      >
        <span class="banner-title">{{ t("onboarding.ready.title") }}</span>
        <span class="banner-description">{{ t("onboarding.ready.description") }}</span>
      </output>
      <div
        v-else-if="
          config.onboardingState.value.state === 'blocked' && config.onboardingState.value.recoveryKind
        "
        class="onboarding-banner onboarding-banner-blocked"
        role="alert"
        aria-live="assertive"
      >
        <p class="banner-title">
          {{ t(`onboarding.recovery.${config.onboardingState.value.recoveryKind}.title`) }}
        </p>
        <p>{{ t(`onboarding.recovery.${config.onboardingState.value.recoveryKind}.description`) }}</p>
      </div>
      <output v-if="config.quickPairState.value === 'validating' || config.quickPairState.value === 'pairing'" class="quick-pair-state" aria-live="polite" aria-atomic="true">
        <span>{{ t("auth.quickPairValidating") }}</span>
      </output>
      <output v-else-if="config.quickPairState.value === 'connecting'" class="quick-pair-state" aria-live="polite" aria-atomic="true">
        <span>{{ t("auth.quickPairConnecting") }}</span>
      </output>
      <div v-else>
        <p v-if="config.quickPairState.value === 'failed'" class="error" role="alert" aria-live="assertive" aria-atomic="true">{{ t("auth.quickPairFailed") }}</p>
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

    <!-- Config page (existing content) -->
    <template v-if="currentPage === 'config'">
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

      <WebSearchSettings
        :model-value="config.form"
        :disabled="!config.canSave.value"
        :saving="config.sectionSaving['web-search']"
        @update:model-value="Object.assign(config.form, $event)"
        @save="config.saveSection('web-search')"
      />

      <BrowserSettings
        :model-value="config.form"
        :disabled="!config.canSave.value"
        :saving="config.sectionSaving.browser"
        @update:model-value="Object.assign(config.form, $event)"
        @save="config.saveSection('browser')"
      />

      <ComposioSettings
        :model-value="config.form"
        :disabled="!config.canSave.value"
        :saving="config.sectionSaving.composio"
        @update:model-value="Object.assign(config.form, $event)"
        @save="config.saveSection('composio')"
      />

      <MemorySettings
        :model-value="config.form"
        :disabled="!config.canSave.value"
        :saving="config.sectionSaving.memory"
        @update:model-value="Object.assign(config.form, $event)"
        @save="config.saveSection('memory')"
      />

      <ChannelsOverview
        v-if="config.isOperatorReady.value"
        :gateway-url="config.baseUrl.value"
        :bearer-token="config.bearerToken.value"
      />

      <SchedulerStatus
        v-if="config.isOperatorReady.value"
        :gateway-url="config.baseUrl.value"
        :bearer-token="config.bearerToken.value"
      />

      <CostOverview
        v-if="config.isOperatorReady.value"
        :gateway-url="config.baseUrl.value"
        :bearer-token="config.bearerToken.value"
      />

      <McpOverview
        v-if="config.isOperatorReady.value"
        :gateway-url="config.baseUrl.value"
        :bearer-token="config.bearerToken.value"
      />

      <TunnelOverview
        v-if="config.isOperatorReady.value"
        :gateway-url="config.baseUrl.value"
        :bearer-token="config.bearerToken.value"
      />

      <ReliabilityOverview
        v-if="config.isOperatorReady.value"
        :gateway-url="config.baseUrl.value"
        :bearer-token="config.bearerToken.value"
      />

      <HeartbeatOverview
        v-if="config.isOperatorReady.value"
        :gateway-url="config.baseUrl.value"
        :bearer-token="config.bearerToken.value"
      />

      <HealthDashboard
        v-if="config.isOperatorReady.value"
        :gateway-url="config.baseUrl.value"
        :bearer-token="config.bearerToken.value"
      />

      <!-- TODO: Wire UpdateSettings when raw AdminConfigView is exposed from useConfig
           (UpdateSettings expects AdminConfigView, not AdminConfigForm) -->
    </template>

    <!-- Sessions page -->
    <template v-if="currentPage === 'sessions' && config.isOperatorReady.value">
      <section class="card">
        <h2>{{ t("nav.sessions", "Sessions") }}</h2>
        <SessionFilters
          @update:status="sessionStatusFilter = $event"
          @update:sort="sessionSort = $event"
        />
        <div class="sessions-layout">
          <SessionList
            :gateway-url="adminGatewayUrl"
            :auth-headers="adminAuthHeaders"
            :status-filter="sessionStatusFilter"
            :sort="sessionSort"
            @select="onSelectSession"
          />
          <SessionDetail
            v-if="selectedSession"
            :gateway-url="adminGatewayUrl"
            :auth-headers="adminAuthHeaders"
            :session-id="selectedSession.id"
            @close="selectedSession = null"
            @view-memory="onViewSessionMemory"
          />
        </div>
      </section>
    </template>

    <!-- Memory page -->
    <template v-if="currentPage === 'memory' && config.isOperatorReady.value">
      <section class="card">
        <h2>{{ t("nav.memory", "Memory") }}</h2>
        <MemoryStats
          :gateway-url="adminGatewayUrl"
          :auth-headers="adminAuthHeaders"
        />
      </section>
      <section class="card">
        <MemoryFilters
          :initial-session-id="memorySessionIdFilter"
          @update:category="memoryCategoryFilter = $event"
          @update:session-id="memorySessionIdFilter = $event"
          @update:search="memorySearchFilter = $event"
        />
        <MemoryList
          :gateway-url="adminGatewayUrl"
          :auth-headers="adminAuthHeaders"
          :category-filter="memoryCategoryFilter"
          :session-id-filter="memorySessionIdFilter"
          :search-filter="memorySearchFilter"
        />
      </section>
    </template>

    <section v-if="currentPage === 'config'" class="card">
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

.nav-tabs {
  display: flex;
  gap: 4px;
  margin-top: 12px;
  border-top: 1px solid var(--color-border);
  padding-top: 12px;
}

.nav-tab {
  padding: 6px 16px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  font-family: inherit;
  transition: background 0.15s, color 0.15s;
}

.nav-tab:hover {
  background: color-mix(in srgb, var(--color-bg-secondary) 60%, transparent);
}

.nav-tab-active {
  background: var(--color-bg-input);
  color: var(--color-text-primary);
  border-color: color-mix(in srgb, #3b82f6 45%, var(--color-border));
}

.sessions-layout {
  display: grid;
  grid-template-columns: 1fr;
  gap: 16px;
}

@media (min-width: 768px) {
  .sessions-layout {
    grid-template-columns: 1fr 360px;
  }
}

h2 {
  margin: 0 0 12px;
  font-size: 16px;
}

h3 {
  margin: 0;
  font-size: 14px;
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

.onboarding-intro {
  margin-top: 0;
}

.onboarding-steps {
  list-style: none;
  padding: 0;
  margin: 16px 0;
  display: grid;
  gap: 10px;
}

.onboarding-step {
  border: 1px solid var(--color-border);
  border-radius: 14px;
  padding: 12px;
  background: color-mix(in srgb, var(--color-bg-secondary) 82%, transparent);
}

.onboarding-step-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.onboarding-step p,
.onboarding-banner span {
  display: block;
  margin: 6px 0 0;
}

.step-badge {
  flex-shrink: 0;
  border-radius: 999px;
  padding: 4px 10px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  background: var(--color-bg-input);
  color: var(--color-text-secondary);
}

.onboarding-step[data-step-status="complete"] {
  border-color: color-mix(in srgb, #22c55e 45%, var(--color-border));
}

.onboarding-step[data-step-status="current"] {
  border-color: color-mix(in srgb, #3b82f6 45%, var(--color-border));
}

.onboarding-step[data-step-status="blocked"] {
  border-color: color-mix(in srgb, #ef4444 45%, var(--color-border));
}

.onboarding-banner {
  border-radius: 14px;
  padding: 12px;
  margin: 0 0 16px;
}

.onboarding-banner-ready {
  border: 1px solid color-mix(in srgb, #22c55e 45%, var(--color-border));
  background: color-mix(in srgb, #22c55e 10%, var(--color-bg-secondary));
}

.onboarding-banner-blocked {
  border: 1px solid color-mix(in srgb, #ef4444 45%, var(--color-border));
  background: color-mix(in srgb, #ef4444 8%, var(--color-bg-secondary));
}

.banner-title {
  display: block;
  font-weight: 600;
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

.quick-pair-state span {
  display: block;
  margin: 10px 0;
  font-size: 14px;
  color: var(--color-text-secondary);
}
</style>
