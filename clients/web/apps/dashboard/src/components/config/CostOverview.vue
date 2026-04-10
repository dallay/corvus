<script setup lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import { Button } from "@corvus/ui";
import { computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useCostGovernance } from "@/composables/useCostGovernance";

const props = defineProps<{
  gatewayUrl: string;
  bearerToken: string;
}>();

const { t } = useI18n();

const governance = useCostGovernance(
  () => props.gatewayUrl,
  () => props.bearerToken,
  (key) => t(key)
);

const config = governance.config;
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const summary = governance.summary;
const history = governance.history;
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const loading = governance.loading;
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const error = governance.error;
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const usageUnavailable = governance.usageUnavailable;
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const usageError = governance.usageError;
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const actionMessage = governance.actionMessage;
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const actionError = governance.actionError;
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const actionPending = governance.actionPending;
const hasOperationalData = governance.hasOperationalData;
const activeBudgetState = governance.activeBudgetState;
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const grantOverride = governance.grantOverride;
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const resetSession = governance.resetSession;

const currencyFmt = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
});

const integerFmt = new Intl.NumberFormat("en-US");

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function formatCurrency(value: number | null | undefined): string {
  return currencyFmt.format(value ?? 0);
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function formatCount(value: number | null | undefined): string {
  return integerFmt.format(value ?? 0);
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function formatPercent(value: number | null | undefined): string {
  return `${Math.round(value ?? 0)}%`;
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const statusTone = computed(() => {
  switch (activeBudgetState.value) {
    case "warning":
      return "warning";
    case "exceeded":
      return "exceeded";
    default:
      return "allowed";
  }
});

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const statusLabel = computed(() => {
  switch (activeBudgetState.value) {
    case "warning":
      return t("cost.statusWarning");
    case "exceeded":
      return t("cost.statusExceeded");
    default:
      return t("cost.statusAllowed");
  }
});

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const stateTestId = computed(() => `cost-state-${activeBudgetState.value}`);

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const maxHistoryCost = computed(() => {
  const points = history.value?.points ?? [];
  return points.reduce((highest, point) => Math.max(highest, point.cost_usd), 0);
});

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const historyPoints = computed(() => history.value?.points ?? []);

const showOverrideAction = computed(
  () => hasOperationalData.value && config.value?.allow_override === true
);

const showResetAction = computed(() => hasOperationalData.value);

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const showActionPanel = computed(() => showOverrideAction.value || showResetAction.value);

function refreshGovernance(): void {
  governance.reload().catch(() => undefined);
}

refreshGovernance();

watch(
  () => [props.gatewayUrl, props.bearerToken],
  () => {
    refreshGovernance();
  },
  { deep: false }
);
</script>

<template>
  <section class="card">
    <h2>{{ t("sections.cost") }}</h2>

    <p v-if="loading" class="helper" aria-live="polite">
      {{ t("cost.loading") }}
    </p>
    <p v-else-if="error" class="error" aria-live="assertive">
      {{ error }}
    </p>
    <div v-else-if="config" class="panel" data-testid="cost-overview">
      <div class="status-grid">
        <div class="status-item">
          <span class="status-label">{{ t("cost.enabled") }}</span>
          <span
            class="status-indicator"
            :class="config.enabled ? 'configured' : 'not-configured'"
            aria-hidden="true"
          />
          <span class="status-value">{{ config.enabled ? t("cost.yes") : t("cost.no") }}</span>
        </div>
        <div class="status-item">
          <span class="status-label">{{ t("cost.sessionLimit") }}</span>
          <span class="status-value">{{ formatCurrency(config.session_limit_usd) }}</span>
        </div>
        <div class="status-item">
          <span class="status-label">{{ t("cost.dailyLimit") }}</span>
          <span class="status-value">{{ formatCurrency(config.daily_limit_usd) }}</span>
        </div>
        <div class="status-item">
          <span class="status-label">{{ t("cost.monthlyLimit") }}</span>
          <span class="status-value">{{ formatCurrency(config.monthly_limit_usd) }}</span>
        </div>
        <div class="status-item">
          <span class="status-label">{{ t("cost.warnAtPercent") }}</span>
          <span class="status-value">{{ config.warn_at_percent }}%</span>
        </div>
        <div class="status-item">
          <span class="status-label">{{ t("cost.allowOverride") }}</span>
          <span class="status-value">{{ config.allow_override ? t("cost.yes") : t("cost.no") }}</span>
        </div>
      </div>

      <div v-if="summary" class="live-panel" data-testid="cost-live-summary">
        <div class="state-banner" :class="`state-banner--${statusTone}`" :data-testid="stateTestId">
          <div>
            <p class="section-label">{{ t("cost.liveStatus") }}</p>
            <strong>{{ statusLabel }}</strong>
          </div>
          <span class="state-period">{{ summary.period ?? t("cost.periodStable") }}</span>
        </div>

        <div class="metric-grid">
          <article class="metric-card">
            <span class="metric-label">{{ t("cost.sessionSpend") }}</span>
            <strong>{{ formatCurrency(summary.session_cost_usd) }}</strong>
          </article>
          <article class="metric-card">
            <span class="metric-label">{{ t("cost.dailySpend") }}</span>
            <strong>{{ formatCurrency(summary.daily_cost_usd) }}</strong>
          </article>
          <article class="metric-card">
            <span class="metric-label">{{ t("cost.monthlySpend") }}</span>
            <strong>{{ formatCurrency(summary.monthly_cost_usd) }}</strong>
          </article>
          <article class="metric-card">
            <span class="metric-label">{{ t("cost.requests") }}</span>
            <strong>{{ formatCount(summary.request_count) }}</strong>
          </article>
          <article class="metric-card">
            <span class="metric-label">{{ t("cost.tokens") }}</span>
            <strong>{{ formatCount(summary.total_tokens) }}</strong>
          </article>
        </div>

        <div class="progress-stack">
          <div class="progress-card">
            <div class="progress-header">
              <span>{{ t("cost.sessionBudgetUsage") }}</span>
              <strong>{{ formatPercent(summary?.percent_used_session ?? 0) }}</strong>
            </div>
            <div class="progress-bar" aria-hidden="true">
              <span :style="{ width: `${Math.min(summary?.percent_used_session ?? 0, 100)}%` }" />
            </div>
          </div>
          <div class="progress-card">
            <div class="progress-header">
              <span>{{ t("cost.dailyBudgetUsage") }}</span>
              <strong>{{ formatPercent(summary.percent_used_daily) }}</strong>
            </div>
            <div class="progress-bar" aria-hidden="true">
              <span :style="{ width: `${Math.min(summary.percent_used_daily, 100)}%` }" />
            </div>
          </div>
          <div class="progress-card">
            <div class="progress-header">
              <span>{{ t("cost.monthlyBudgetUsage") }}</span>
              <strong>{{ formatPercent(summary.percent_used_monthly) }}</strong>
            </div>
            <div class="progress-bar" aria-hidden="true">
              <span :style="{ width: `${Math.min(summary.percent_used_monthly, 100)}%` }" />
            </div>
          </div>
        </div>
      </div>

      <p v-if="usageUnavailable" class="helper" data-testid="cost-config-fallback" aria-live="polite">
        {{ usageError ?? t("cost.usageUnavailable") }}
      </p>

      <div v-if="historyPoints.length > 0" class="history-panel" data-testid="cost-history">
        <div class="panel-header">
          <h3>{{ t("cost.history") }}</h3>
          <span class="history-total">{{ formatCurrency(history?.totals.cost_usd) }}</span>
        </div>
        <ul class="history-list">
          <li v-for="point in historyPoints" :key="point.bucket" class="history-row">
            <div class="history-meta">
              <strong>{{ point.bucket }}</strong>
              <span>
                {{ formatCount(point.requests) }} · {{ formatCount(point.tokens) }}
              </span>
            </div>
            <div class="history-bar-wrap">
              <div class="history-bar" aria-hidden="true">
                <span
                  :style="{
                    width: `${maxHistoryCost > 0 ? (point.cost_usd / maxHistoryCost) * 100 : 0}%`,
                  }"
                />
              </div>
              <span class="history-cost">{{ formatCurrency(point.cost_usd) }}</span>
            </div>
          </li>
        </ul>
      </div>

      <div v-if="showActionPanel" class="actions-panel" data-testid="cost-actions">
        <div class="panel-header">
          <h3>{{ t("cost.actions") }}</h3>
        </div>
        <div class="action-row">
          <Button
            v-if="showOverrideAction"
            variant="secondary"
            size="sm"
            :disabled="actionPending"
            data-testid="cost-action-override"
            @click="grantOverride"
          >
            {{ t("cost.grantOverride") }}
          </Button>
          <Button
            v-if="showResetAction"
            variant="secondary"
            size="sm"
            :disabled="actionPending"
            data-testid="cost-action-reset-session"
            @click="resetSession"
          >
            {{ t("cost.resetSession") }}
          </Button>
        </div>
        <p v-if="actionMessage" class="helper" aria-live="polite">
          {{ actionMessage }}
        </p>
        <p v-if="actionError" class="error" aria-live="assertive">
          {{ actionError }}
        </p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.panel,
.live-panel,
.history-panel,
.actions-panel {
  display: grid;
  gap: 12px;
}

.status-grid,
.metric-grid {
  display: grid;
  gap: 8px;
}

.status-item,
.metric-card,
.progress-card,
.history-panel,
.actions-panel,
.state-banner {
  border: 1px solid var(--corvus-color-border-default);
  border-radius: 10px;
  background: var(--corvus-color-bg-surface);
}

.status-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
}

.status-indicator {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.configured {
  background: var(--corvus-color-status-success);
}

.not-configured {
  background: var(--corvus-color-text-disabled);
}

.status-label,
.metric-label,
.section-label,
.progress-header span,
.history-meta span,
.history-total,
.state-period {
  font-size: 12px;
  color: var(--corvus-color-text-secondary);
}

.status-label {
  font-weight: 500;
  flex: 1;
}

.status-value,
.metric-card strong,
.history-cost,
.progress-header strong {
  font-size: 13px;
}

.state-banner,
.panel-header,
.action-row,
.history-bar-wrap,
.progress-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.state-banner,
.actions-panel,
.history-panel {
  padding: 12px;
}

.state-banner--allowed {
  border-color: var(--corvus-color-status-success);
}

.state-banner--warning {
  border-color: var(--corvus-color-status-warning);
}

.state-banner--exceeded {
  border-color: var(--corvus-color-status-error);
}

.metric-grid {
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
}

.metric-card,
.progress-card {
  padding: 12px;
}

.metric-card {
  display: grid;
  gap: 6px;
}

.progress-stack,
.history-list {
  display: grid;
  gap: 10px;
}

.progress-bar,
.history-bar {
  overflow: hidden;
  height: 8px;
  border-radius: 999px;
  background: var(--corvus-color-bg-raised);
}

.progress-bar span,
.history-bar span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--corvus-color-accent-default);
}

.history-list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.history-row {
  display: grid;
  gap: 8px;
}

.history-meta {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
}

.history-bar-wrap {
  gap: 10px;
}

.history-bar {
  flex: 1;
}

.action-row {
  justify-content: flex-start;
  flex-wrap: wrap;
}

@media (max-width: 640px) {
  .history-meta,
  .history-bar-wrap,
  .state-banner,
  .panel-header {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
