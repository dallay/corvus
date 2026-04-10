<script lang="ts" setup>
import { trimTrailingSlashes } from "@corvus/shared";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import { Button, Input } from "@corvus/ui";
import { ref } from "vue";
import { useI18n } from "vue-i18n";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import ChatWorkspace from "@/components/chat/ChatWorkspace.vue";
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
import CerebroObservationDetail from "@/components/memory/CerebroObservationDetail.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import CerebroSearchPanel from "@/components/memory/CerebroSearchPanel.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import CerebroTimelinePanel from "@/components/memory/CerebroTimelinePanel.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import LocalMemoryExplorerPanel from "@/components/memory/LocalMemoryExplorerPanel.vue";
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
import type {
  AdminCerebroSearchResult,
  AdminSessionView,
  LocalMemoryExplorerSelection,
  LocalMemorySubview,
} from "@/types/admin-sessions";

const { t } = useI18n();

const config = useConfig(t);

type DashboardPage = "config" | "sessions" | "memory" | "chat";
const currentPage = ref<DashboardPage>("config");

// Session view state
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const sessionStatusFilter = ref<"active" | "ended" | undefined>(undefined);
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const sessionSort = ref<"last_activity" | "started_at">("last_activity");
const selectedSession = ref<AdminSessionView | null>(null);

// Memory view state
const memoryCategoryFilter = ref<string | undefined>(undefined);
const memorySessionIdFilter = ref<string | undefined>(undefined);
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const memorySearchFilter = ref<string | undefined>(undefined);
const memoryMode = ref<"local" | "cerebro">("local");
const localMemorySubview = ref<LocalMemorySubview>("browse");
const localExplorerSelection = ref<LocalMemoryExplorerSelection>({});
const selectedCerebroResult = ref<AdminCerebroSearchResult | null>(null);

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
  memoryMode.value = "local";
  localMemorySubview.value = "browse";
  currentPage.value = "memory";
  selectedSession.value = null;
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function onSelectCerebroResult(result: AdminCerebroSearchResult) {
  selectedCerebroResult.value = result;
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function openLocalExplorer(selection: LocalMemoryExplorerSelection = {}) {
  if (selection.category !== undefined) {
    memoryCategoryFilter.value = selection.category;
  }
  if (selection.sessionId !== undefined) {
    memorySessionIdFilter.value = selection.sessionId;
  }

  // Merge drill-in state so explorer clicks can add category/session context without wiping
  // an existing entry focus. That keeps operators anchored while they pivot inside the explorer.
  localExplorerSelection.value = {
    ...localExplorerSelection.value,
    ...selection,
  };
  memoryMode.value = "local";
  currentPage.value = "memory";
  localMemorySubview.value = "explorer";
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function openLocalBrowse(selection: LocalMemoryExplorerSelection = localExplorerSelection.value) {
  if (selection.category !== undefined) {
    memoryCategoryFilter.value = selection.category;
  }
  if (selection.sessionId !== undefined) {
    memorySessionIdFilter.value = selection.sessionId;
  }
  // Replace the explorer selection when returning to browse so the list reflects one clean
  // filter snapshot instead of preserving stale drill-in context that no longer matters there.
  localExplorerSelection.value = {
    ...selection,
  };
  localMemorySubview.value = "browse";
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function onLocalExplorerSelectionChange(selection: LocalMemoryExplorerSelection) {
  localExplorerSelection.value = selection;
}
</script>

<template>
  <main class="dashboard-shell">
    <header class="header-card">
      <div class="header-title-row">
        <img alt="Corvus" class="header-logo" height="32" src="/favicon-light.svg" width="32"/>
        <div>
          <h1>{{ t("app.title") }}</h1>
          <p>{{ t("app.subtitle") }}</p>
        </div>
      </div>
      <nav v-if="config.isOperatorReady.value" aria-label="Dashboard navigation" class="nav-tabs"
           role="tablist">
        <button
            :aria-selected="currentPage === 'config'"
            :class="{ 'nav-tab-active': currentPage === 'config' }"
            class="nav-tab"
            data-testid="nav-config"
            role="tab"
            @click="currentPage = 'config'"
        >
          {{ t("nav.config", "Config") }}
        </button>
        <button
            :aria-selected="currentPage === 'sessions'"
            :class="{ 'nav-tab-active': currentPage === 'sessions' }"
            class="nav-tab"
            data-testid="nav-sessions"
            role="tab"
            @click="currentPage = 'sessions'"
        >
          {{ t("nav.sessions", "Sessions") }}
        </button>
        <button
            :aria-selected="currentPage === 'memory'"
            :class="{ 'nav-tab-active': currentPage === 'memory' }"
            class="nav-tab"
            data-testid="nav-memory"
            role="tab"
            @click="currentPage = 'memory'"
        >
          {{ t("nav.memory", "Memory") }}
        </button>
        <button
            :aria-selected="currentPage === 'chat'"
            :class="{ 'nav-tab-active': currentPage === 'chat' }"
            class="nav-tab"
            data-testid="nav-chat"
            role="tab"
            @click="currentPage = 'chat'"
        >
          {{ t("nav.chat", "Chat") }}
        </button>
      </nav>
    </header>

    <!-- Auth / Onboarding section — always visible -->
    <section class="card">
      <h2>{{ t("sections.auth") }}</h2>
      <p class="helper onboarding-intro">{{ t("onboarding.intro") }}</p>
      <ol aria-label="Dashboard onboarding steps" class="onboarding-steps">
        <li
            v-for="step in config.onboardingSteps.value"
            :key="step.key"
            :data-step-status="step.status"
            class="onboarding-step"
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
          aria-live="polite"
          class="onboarding-banner onboarding-banner-ready"
      >
        <span class="banner-title">{{ t("onboarding.ready.title") }}</span>
        <span class="banner-description">{{ t("onboarding.ready.description") }}</span>
      </output>
      <div
          v-else-if="
          config.onboardingState.value.state === 'blocked' && config.onboardingState.value.recoveryKind
        "
          aria-live="assertive"
          class="onboarding-banner onboarding-banner-blocked"
          role="alert"
      >
        <p class="banner-title">
          {{ t(`onboarding.recovery.${config.onboardingState.value.recoveryKind}.title`) }}
        </p>
        <p>{{
            t(`onboarding.recovery.${config.onboardingState.value.recoveryKind}.description`)
          }}</p>
      </div>
      <output
          v-if="config.quickPairState.value === 'validating' || config.quickPairState.value === 'pairing'"
          aria-atomic="true" aria-live="polite" class="quick-pair-state">
        <span>{{ t("auth.quickPairValidating") }}</span>
      </output>
      <output v-else-if="config.quickPairState.value === 'connecting'" aria-atomic="true"
              aria-live="polite" class="quick-pair-state">
        <span>{{ t("auth.quickPairConnecting") }}</span>
      </output>
      <div v-else>
        <p v-if="config.quickPairState.value === 'failed'" aria-atomic="true" aria-live="assertive"
           class="error" role="alert">{{ t("auth.quickPairFailed") }}</p>
        <div class="grid">
          <label>
            <span>{{ t("auth.baseUrl") }}</span>
            <Input v-model="config.baseUrl.value" :placeholder="t('form.baseUrlPlaceholder')"/>
          </label>
          <label>
            <span>{{ t("auth.pairingCode") }}</span>
            <Input v-model="config.pairingCode.value" type="password"/>
          </label>
          <label>
            <span>{{ t("auth.bearerToken") }}</span>
            <Input v-model="config.bearerToken.value" type="password"/>
          </label>
        </div>
        <div class="actions">
          <Button :disabled="config.loading.value" @click="config.pairGateway">{{
              t("auth.pair")
            }}
          </Button>
          <Button :disabled="config.loading.value" variant="secondary"
                  @click="config.connectGateway">
            {{ t("auth.connect") }}
          </Button>
        </div>
      </div>
    </section>

    <!-- Config page (existing content) -->
    <template v-if="currentPage === 'config'">
      <section v-if="config.isOperatorReady.value" class="overview-section">
        <div class="section-intro">
          <p class="section-kicker">Operational overview</p>
          <h2>Current system state</h2>
          <p class="helper section-copy">
            Review runtime, scheduler, gateway, and reliability signals before changing
            configuration.
          </p>
        </div>
        <div class="overview-grid">
          <SchedulerStatus
              :bearer-token="config.bearerToken.value"
              :gateway-url="config.baseUrl.value"
          />

          <CostOverview
              :bearer-token="config.bearerToken.value"
              :gateway-url="config.baseUrl.value"
          />

          <TunnelOverview
              :bearer-token="config.bearerToken.value"
              :gateway-url="config.baseUrl.value"
          />

          <ReliabilityOverview
              :bearer-token="config.bearerToken.value"
              :gateway-url="config.baseUrl.value"
          />

          <HeartbeatOverview
              :bearer-token="config.bearerToken.value"
              :gateway-url="config.baseUrl.value"
          />

          <McpOverview
              :bearer-token="config.bearerToken.value"
              :gateway-url="config.baseUrl.value"
          />

          <ChannelsOverview
              :bearer-token="config.bearerToken.value"
              :gateway-url="config.baseUrl.value"
          />

          <HealthDashboard
              :bearer-token="config.bearerToken.value"
              :gateway-url="config.baseUrl.value"
          />
        </div>
      </section>

      <section class="section-intro-card">
        <div class="section-intro">
          <p class="section-kicker">Configuration surfaces</p>
          <h2>System controls</h2>
          <p class="helper section-copy">
            Adjust operators, runtime, security, and integrations in grouped technical panels.
          </p>
        </div>
      </section>

      <div class="config-stack">
        <GeneralSettings
            :disabled="!config.canSave.value"
            :memory-backend-options="config.memoryBackendOptions.value"
            :model-value="config.form"
            :saving="config.sectionSaving.general"
            @save="config.saveSection('general')"
            @update:model-value="Object.assign(config.form, $event)"
        />

        <SecuritySettings
            :autonomy-level-options="config.autonomyLevelOptions.value"
            :disabled="!config.canSave.value"
            :model-value="config.form"
            :saving="config.sectionSaving.security"
            @save="config.saveSection('security')"
            @update:model-value="Object.assign(config.form, $event)"
        />

        <ObservabilitySettings
            :disabled="!config.canSave.value"
            :model-value="config.form"
            :observability-backend-options="config.observabilityBackendOptions.value"
            :saving="config.sectionSaving.observability"
            @save="config.saveSection('observability')"
            @update:model-value="Object.assign(config.form, $event)"
        />

        <RuntimeSettings
            :disabled="!config.canSave.value"
            :model-value="config.form"
            :runtime-kind-options="config.runtimeKindOptions.value"
            :saving="config.sectionSaving.runtime"
            @save="config.saveSection('runtime')"
            @update:model-value="Object.assign(config.form, $event)"
        />

        <SchedulerSettings
            :disabled="!config.canSave.value"
            :model-value="config.form"
            :saving="config.sectionSaving.scheduler"
            @save="config.saveSection('scheduler')"
            @update:model-value="Object.assign(config.form, $event)"
        />

        <GatewaySettings
            :disabled="!config.canSave.value"
            :model-value="config.form"
            :saving="config.sectionSaving.gateway"
            @save="config.saveSection('gateway')"
            @update:model-value="Object.assign(config.form, $event)"
        />

        <WebhookSettings
            :disabled="!config.canSave.value"
            :model-value="config.form"
            :saving="config.sectionSaving.webhook"
            @save="config.saveSection('webhook')"
            @update:model-value="Object.assign(config.form, $event)"
        />

        <WebSearchSettings
            :disabled="!config.canSave.value"
            :model-value="config.form"
            :saving="config.sectionSaving['web-search']"
            @save="config.saveSection('web-search')"
            @update:model-value="Object.assign(config.form, $event)"
        />

        <BrowserSettings
            :disabled="!config.canSave.value"
            :model-value="config.form"
            :saving="config.sectionSaving.browser"
            @save="config.saveSection('browser')"
            @update:model-value="Object.assign(config.form, $event)"
        />

        <ComposioSettings
            :disabled="!config.canSave.value"
            :model-value="config.form"
            :saving="config.sectionSaving.composio"
            @save="config.saveSection('composio')"
            @update:model-value="Object.assign(config.form, $event)"
        />

        <MemorySettings
            :disabled="!config.canSave.value"
            :model-value="config.form"
            :saving="config.sectionSaving.memory"
            @save="config.saveSection('memory')"
            @update:model-value="Object.assign(config.form, $event)"
        />
      </div>

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
              :auth-headers="adminAuthHeaders"
              :gateway-url="adminGatewayUrl"
              :sort="sessionSort"
              :status-filter="sessionStatusFilter"
              @select="onSelectSession"
          />
          <SessionDetail
              v-if="selectedSession"
              :auth-headers="adminAuthHeaders"
              :gateway-url="adminGatewayUrl"
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
            :auth-headers="adminAuthHeaders"
            :gateway-url="adminGatewayUrl"
            @select-category="openLocalExplorer({ category: $event })"
        />
      </section>
      <section class="card">
        <div aria-label="Memory mode" class="memory-mode-tabs" role="tablist">
          <button
              :aria-selected="memoryMode === 'local'"
              :class="{ 'nav-tab-active': memoryMode === 'local' }"
              class="nav-tab"
              data-testid="memory-mode-local"
              role="tab"
              @click="memoryMode = 'local'; selectedCerebroResult = null"
          >
            Local Memory
          </button>
          <button
              :aria-selected="memoryMode === 'cerebro'"
              :class="{ 'nav-tab-active': memoryMode === 'cerebro' }"
              class="nav-tab"
              data-testid="memory-mode-cerebro"
              role="tab"
              @click="memoryMode = 'cerebro'"
          >
            Cerebro Memory
          </button>
        </div>

        <template v-if="memoryMode === 'local'">
          <MemoryFilters
              :initial-session-id="memorySessionIdFilter"
              @update:category="memoryCategoryFilter = $event"
              @update:session-id="memorySessionIdFilter = $event"
              @update:search="memorySearchFilter = $event"
          />
          <div aria-label="Local memory workspace" class="memory-mode-tabs" role="tablist">
            <button
                :aria-selected="localMemorySubview === 'browse'"
                :class="{ 'nav-tab-active': localMemorySubview === 'browse' }"
                class="nav-tab"
                data-testid="local-memory-browse"
                role="tab"
                @click="localMemorySubview = 'browse'"
            >
              Browse
            </button>
            <button
                :aria-selected="localMemorySubview === 'explorer'"
                :class="{ 'nav-tab-active': localMemorySubview === 'explorer' }"
                class="nav-tab"
                data-testid="local-memory-explorer"
                role="tab"
                @click="localMemorySubview = 'explorer'"
            >
              Explorer
            </button>
          </div>

          <MemoryList
              v-if="localMemorySubview === 'browse'"
              :auth-headers="adminAuthHeaders"
              :category-filter="memoryCategoryFilter"
              :gateway-url="adminGatewayUrl"
              :search-filter="memorySearchFilter"
              :session-id-filter="memorySessionIdFilter"
              @select-category="openLocalExplorer({ category: $event })"
              @select-session="openLocalExplorer({ sessionId: $event })"
              @open-explorer="openLocalExplorer($event)"
          />
          <LocalMemoryExplorerPanel
              v-else
              :auth-headers="adminAuthHeaders"
              :gateway-url="adminGatewayUrl"
              :selection="localExplorerSelection"
              @selection-change="onLocalExplorerSelectionChange"
              @open-browse="openLocalBrowse($event)"
          />
        </template>

        <div v-else class="cerebro-memory-layout">
          <CerebroSearchPanel
              :auth-headers="adminAuthHeaders"
              :gateway-url="adminGatewayUrl"
              :status="null"
              @select="onSelectCerebroResult"
          />
          <CerebroObservationDetail
              :auth-headers="adminAuthHeaders"
              :gateway-url="adminGatewayUrl"
              :selected="selectedCerebroResult"
          />
          <CerebroTimelinePanel
              :auth-headers="adminAuthHeaders"
              :gateway-url="adminGatewayUrl"
              :selected="selectedCerebroResult"
          />
        </div>
      </section>
    </template>

    <!-- Chat page -->
    <template v-if="currentPage === 'chat' && config.isOperatorReady.value">
      <section class="chat-section">
        <ChatWorkspace :config="config" />
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
  display: grid;
  gap: 24px;
  margin: 0 auto;
  max-width: 1180px;
  padding: 24px 24px 64px;
}

.header-card,
.card {
  background: var(--corvus-color-bg-surface);
  border: 1px solid var(--corvus-color-border-default);
  border-radius: var(--corvus-radius-card-lg);
  padding: 18px;
}

.header-card {
  padding: 20px 22px;
}

.header-card h1 {
  font-size: clamp(28px, 4vw, 40px);
  letter-spacing: -0.03em;
  line-height: 0.98;
  margin: 0;
}

.header-card p {
  color: var(--corvus-color-text-secondary);
  margin: 6px 0 0;
}

.header-title-row {
  align-items: center;
  display: flex;
  gap: 12px;
}

.header-logo {
  flex-shrink: 0;
}

.nav-tabs {
  border-top: 1px solid var(--corvus-color-border-default);
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 16px;
  padding-top: 16px;
}

.memory-mode-tabs {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.cerebro-memory-layout {
  display: grid;
  gap: 12px;
}

.nav-tab {
  background: transparent;
  border: 1px solid var(--corvus-color-border-default);
  border-radius: var(--corvus-radius-pill);
  color: var(--corvus-color-text-secondary);
  cursor: pointer;
  font-family: inherit;
  font-size: 13px;
  font-weight: 500;
  padding: 8px 16px;
  transition: background var(--corvus-motion-duration-micro) var(--corvus-motion-easing-default),
  color var(--corvus-motion-duration-micro) var(--corvus-motion-easing-default);
}

.nav-tab:hover {
  background: var(--corvus-color-bg-raised);
}

.nav-tab-active {
  background: var(--corvus-color-bg-raised);
  border-color: var(--corvus-color-interactive-default);
  color: var(--corvus-color-text-primary);
}

.sessions-layout {
  display: grid;
  gap: 20px;
  grid-template-columns: 1fr;
}

@media (min-width: 768px) {
  .sessions-layout {
    grid-template-columns: 1fr 360px;
  }
}

h2 {
  font-size: 20px;
  line-height: 1.1;
  margin: 0 0 14px;
}

h3 {
  font-size: 14px;
  margin: 0;
}

.grid {
  display: grid;
  gap: 12px;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
}

label {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

label span {
  color: var(--corvus-color-text-secondary);
  font-size: 12px;
}

.select-input {
  background: var(--corvus-color-bg-surface);
  border: 1px solid var(--corvus-color-border-default);
  border-radius: var(--corvus-radius-input);
  color: var(--corvus-color-text-primary);
  font-family: inherit;
  height: 42px;
  padding: 0 10px;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 12px;
}

.overview-section,
.config-stack {
  display: grid;
  gap: 20px;
}

.section-intro-card {
  border-top: 1px solid var(--corvus-color-border-default);
  padding-top: 8px;
}

.section-intro {
  display: grid;
  gap: 8px;
  max-width: 60ch;
}

.section-kicker {
  color: var(--corvus-color-text-secondary);
  font-family: var(--corvus-typography-font-mono);
  font-size: 11px;
  letter-spacing: 0.08em;
  margin: 0;
  text-transform: uppercase;
}

.section-copy {
  margin: 0;
}

.overview-grid {
  display: grid;
  gap: 18px;
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

@media (min-width: 1100px) {
  .overview-grid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}

@media (max-width: 799px) {
  .overview-grid {
    grid-template-columns: 1fr;
  }
}

.switch-row {
  align-items: center;
  flex-direction: row;
  gap: 8px;
  margin-top: 20px;
}

.helper,
.ok,
.error {
  font-size: 13px;
  margin: 10px 0 0;
}

.onboarding-intro {
  margin-top: 0;
  max-width: 60ch;
}

.onboarding-steps {
  display: grid;
  gap: 10px;
  list-style: none;
  margin: 16px 0;
  padding: 0;
}

.onboarding-step {
  background: var(--corvus-color-bg-base);
  border: 1px solid var(--corvus-color-border-default);
  border-radius: var(--corvus-radius-card);
  padding: 14px 16px;
}

.onboarding-step-header {
  align-items: flex-start;
  display: flex;
  gap: 12px;
  justify-content: space-between;
}

.onboarding-step p,
.onboarding-banner span {
  display: block;
  margin: 6px 0 0;
}

.step-badge {
  background: var(--corvus-color-bg-raised);
  border-radius: var(--corvus-radius-pill);
  color: var(--corvus-color-text-secondary);
  flex-shrink: 0;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  padding: 4px 10px;
  text-transform: uppercase;
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

.onboarding-banner {
  border-radius: var(--corvus-radius-card);
  margin: 0 0 18px;
  padding: 14px 16px;
}

.onboarding-banner-ready {
  background: var(--corvus-color-bg-surface);
  border: 1px solid var(--corvus-color-status-success);
}

.onboarding-banner-blocked {
  background: var(--corvus-color-bg-surface);
  border: 1px solid var(--corvus-color-status-error);
}

.banner-title {
  display: block;
  font-weight: 600;
}

.helper {
  color: var(--corvus-color-text-secondary);
}

.ok {
  color: var(--corvus-color-status-success);
}

.error {
  color: var(--corvus-color-status-error);
}

.quick-pair-state span {
  color: var(--corvus-color-text-secondary);
  display: block;
  font-size: 14px;
  margin: 10px 0;
}

.chat-section {
  display: flex;
  min-height: 520px;
  max-height: 75vh;
}
</style>
