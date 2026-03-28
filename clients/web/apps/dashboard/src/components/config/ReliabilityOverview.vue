<script setup lang="ts">
import { trimTrailingSlashes, validateGatewayUrl } from "@corvus/shared";
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { AdminReliabilityView } from "@/types/admin-config";

const props = defineProps<{
  gatewayUrl: string;
  bearerToken: string;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();

const reliability = ref<AdminReliabilityView | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);

async function fetchReliability() {
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
    reliability.value = data.config?.reliability ?? null;
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

watch(() => [props.gatewayUrl, props.bearerToken], fetchReliability, { immediate: true });
</script>

<template>
  <section class="card">
    <h2>{{ t("sections.reliability") }}</h2>
    <p v-if="loading" class="helper">{{ t("reliability.loading") }}</p>
    <p v-else-if="error" class="error">{{ error }}</p>
    <div v-else-if="reliability" class="status-grid" data-testid="reliability-overview">
      <div class="status-item">
        <span class="status-label">{{ t("reliability.providerRetries") }}</span>
        <span class="status-value">{{ reliability.provider_retries }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("reliability.providerBackoff") }}</span>
        <span class="status-value">{{ reliability.provider_backoff_ms }}ms</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("reliability.fallbackProviders") }}</span>
        <span class="status-value">{{
          reliability.fallback_providers.length > 0
            ? reliability.fallback_providers.join(", ")
            : t("reliability.none")
        }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("reliability.modelFallbacks") }}</span>
        <span class="status-value">{{
          Object.keys(reliability.model_fallbacks).length > 0
            ? Object.entries(reliability.model_fallbacks).map(([k, v]) => `${k} → ${v}`).join(", ")
            : t("reliability.none")
        }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("reliability.channelInitialBackoff") }}</span>
        <span class="status-value">{{ reliability.channel_initial_backoff_secs }}s</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("reliability.channelMaxBackoff") }}</span>
        <span class="status-value">{{ reliability.channel_max_backoff_secs }}s</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("reliability.schedulerPoll") }}</span>
        <span class="status-value">{{ reliability.scheduler_poll_secs }}s</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("reliability.schedulerRetries") }}</span>
        <span class="status-value">{{ reliability.scheduler_retries }}</span>
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

.status-label {
  font-weight: 500;
  flex: 1;
}

.status-value {
  font-size: 12px;
  color: var(--color-text-secondary);
}
</style>
