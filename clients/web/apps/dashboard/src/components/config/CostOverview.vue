<script setup lang="ts">
import { trimTrailingSlashes, validateGatewayUrl } from "@corvus/shared";
import { computed, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { AdminCostView } from "@/types/admin-config";

const props = defineProps<{
  gatewayUrl: string;
  bearerToken: string;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();

const cost = ref<AdminCostView | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);

const currencyFmt = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
});

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const formattedDaily = computed(() =>
  cost.value != null ? currencyFmt.format(cost.value.daily_limit_usd) : ""
);
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const formattedMonthly = computed(() =>
  cost.value != null ? currencyFmt.format(cost.value.monthly_limit_usd) : ""
);

let abortController: AbortController | undefined;

async function fetchCost(signal?: AbortSignal) {
  loading.value = true;
  error.value = null;
  try {
    const base = validateGatewayUrl(props.gatewayUrl);
    if (!base) {
      cost.value = null;
      error.value = "Invalid gateway URL";
      return;
    }
    const baseStr = trimTrailingSlashes(base.toString());
    const requestUrl = new URL("web/admin/config", `${baseStr}/`);
    const res = await fetch(requestUrl.toString(), {
      headers: { Authorization: `Bearer ${props.bearerToken}` },
      signal,
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json();
    if (data?.config?.cost) {
      cost.value = data.config.cost;
    } else {
      cost.value = null;
      error.value = "Cost data not available";
    }
  } catch (e: unknown) {
    if (e instanceof DOMException && e.name === "AbortError") return;
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

watch(
  () => [props.gatewayUrl, props.bearerToken],
  () => {
    if (abortController) {
      abortController.abort();
    }
    abortController = new AbortController();
    fetchCost(abortController.signal);
  },
  { immediate: true }
);
onUnmounted(() => {
  abortController?.abort();
});
</script>

<template>
  <section class="card">
    <h2>{{ t("sections.cost") }}</h2>
    <p v-if="loading" class="helper" aria-live="polite" role="status">{{ t("cost.loading") }}</p>
    <p v-else-if="error" class="error" aria-live="assertive" role="alert">{{ error }}</p>
    <div v-else-if="cost" class="status-grid" data-testid="cost-overview">
      <div class="status-item">
        <span class="status-label">{{ t("cost.enabled") }}</span>
        <span
          class="status-indicator"
          :class="cost.enabled ? 'configured' : 'not-configured'"
        />
        <span class="status-value">{{
          cost.enabled ? t("cost.yes") : t("cost.no")
        }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("cost.dailyLimit") }}</span>
        <span class="status-value">{{ formattedDaily }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("cost.monthlyLimit") }}</span>
        <span class="status-value">{{ formattedMonthly }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("cost.warnAtPercent") }}</span>
        <span class="status-value">{{ cost.warn_at_percent }}%</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("cost.allowOverride") }}</span>
        <span class="status-value">{{
          cost.allow_override ? t("cost.yes") : t("cost.no")
        }}</span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.status-grid {
  display: grid;
  gap: 8px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-bg-secondary) 82%, transparent);
}

.status-indicator {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.configured {
  background: #22c55e;
}

.not-configured {
  background: #9ca3af;
}

.status-label {
  font-weight: 500;
  flex: 1;
}

.status-value {
  font-size: 12px;
  color: var(--color-text-secondary);
}
</style>
