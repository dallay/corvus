<script setup lang="ts">
import { trimTrailingSlashes, validateGatewayUrl } from "@corvus/shared";
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { AdminHeartbeatView } from "@/types/admin-config";

const props = defineProps<{
  gatewayUrl: string;
  bearerToken: string;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();

const heartbeat = ref<AdminHeartbeatView | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);

async function fetchHeartbeat() {
  loading.value = true;
  error.value = null;
  try {
    const base = validateGatewayUrl(props.gatewayUrl);
    if (!base) {
      throw new Error("Invalid gateway URL");
    }
    const baseStr = trimTrailingSlashes(base.toString());
    const requestUrl = new URL("web/admin/config", `${baseStr}/`);
    const res = await fetch(requestUrl.toString(), {
      headers: { Authorization: `Bearer ${props.bearerToken}` },
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json();
    heartbeat.value = data.config?.heartbeat ?? null;
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

watch(() => [props.gatewayUrl, props.bearerToken], fetchHeartbeat, { immediate: true });
</script>

<template>
  <section class="card">
    <h2>{{ t("sections.heartbeat") }}</h2>
    <p v-if="loading" class="helper" aria-live="polite" role="status">{{ t("heartbeat.loading") }}</p>
    <p v-else-if="error" class="error" aria-live="assertive" role="alert">{{ error }}</p>
    <p v-else-if="!heartbeat" class="helper">{{ t("heartbeat.noData") }}</p>
    <div v-else class="status-grid" data-testid="heartbeat-overview">
      <div class="status-item">
        <span class="status-label">{{ t("heartbeat.enabled") }}</span>
        <span
          class="status-indicator"
          :class="heartbeat.enabled ? 'configured' : 'not-configured'"
          aria-hidden="true"
        />
        <span class="status-value">{{
          heartbeat.enabled ? t("heartbeat.yes") : t("heartbeat.no")
        }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("heartbeat.interval") }}</span>
        <span class="status-value">{{ heartbeat.interval_minutes }} {{ t("heartbeat.unit") }}</span>
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
