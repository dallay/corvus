<script setup lang="ts">
import { trimTrailingSlashes, validateGatewayUrl } from "@corvus/shared";
import { onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { AdminTunnelView } from "@/types/admin-config";

const props = defineProps<{
  gatewayUrl: string;
  bearerToken: string;
}>();

const { t } = useI18n();

const tunnel = ref<AdminTunnelView | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);
let abortController: AbortController | undefined;
let fetchId = 0;

async function fetchTunnel() {
  abortController?.abort();
  abortController = new AbortController();
  const myId = ++fetchId;
  loading.value = true;
  error.value = null;
  try {
    const base = validateGatewayUrl(props.gatewayUrl);
    if (!base) {
      throw new Error(t("errors.invalidGatewayUrl"));
    }
    const baseStr = trimTrailingSlashes(base.toString());
    const requestUrl = new URL("web/admin/config", `${baseStr}/`);
    const res = await fetch(requestUrl.toString(), {
      headers: { Authorization: `Bearer ${props.bearerToken}` },
      signal: abortController.signal,
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json();
    if (myId === fetchId) {
      tunnel.value = data.config?.tunnel ?? null;
    }
  } catch (e: unknown) {
    if (e instanceof DOMException && e.name === "AbortError") return;
    if (myId === fetchId) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  } finally {
    if (myId === fetchId) {
      loading.value = false;
    }
  }
}

watch(() => [props.gatewayUrl, props.bearerToken], fetchTunnel, { immediate: true });
onBeforeUnmount(() => abortController?.abort());
</script>

<template>
  <section class="card">
    <h2>{{ t("sections.tunnel") }}</h2>
    <p v-if="loading" class="helper" aria-live="polite">{{ t("tunnel.loading") }}</p>
    <p v-else-if="error" class="error" aria-live="assertive">{{ error }}</p>
    <p v-else-if="!tunnel" class="helper">{{ t("tunnel.noData") }}</p>
    <div v-else class="status-grid" data-testid="tunnel-overview">
      <div class="status-item">
        <span class="status-label">{{ t("tunnel.provider") }}</span>
        <span class="status-value">{{ tunnel.provider }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("tunnel.cloudflareToken") }}</span>
        <span
          class="status-indicator"
          :class="tunnel.has_cloudflare_token ? 'configured' : 'not-configured'"
          aria-hidden="true"
        />
        <span class="status-value">{{
          tunnel.has_cloudflare_token ? t("tunnel.yes") : t("tunnel.no")
        }}</span>
      </div>
      <div v-if="tunnel.tailscale_funnel != null" class="status-item">
        <span class="status-label">{{ t("tunnel.tailscaleFunnel") }}</span>
        <span class="status-value">{{
          tunnel.tailscale_funnel ? t("tunnel.yes") : t("tunnel.no")
        }}</span>
      </div>
      <div v-if="tunnel.tailscale_hostname" class="status-item">
        <span class="status-label">{{ t("tunnel.tailscaleHostname") }}</span>
        <span class="status-value">{{ tunnel.tailscale_hostname }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("tunnel.ngrokAuthToken") }}</span>
        <span
          class="status-indicator"
          :class="tunnel.has_ngrok_auth_token ? 'configured' : 'not-configured'"
          aria-hidden="true"
        />
        <span class="status-value">{{
          tunnel.has_ngrok_auth_token ? t("tunnel.yes") : t("tunnel.no")
        }}</span>
      </div>
      <div v-if="tunnel.ngrok_domain" class="status-item">
        <span class="status-label">{{ t("tunnel.ngrokDomain") }}</span>
        <span class="status-value">{{ tunnel.ngrok_domain }}</span>
      </div>
      <div v-if="tunnel.custom_health_url" class="status-item">
        <span class="status-label">{{ t("tunnel.customHealthUrl") }}</span>
        <span class="status-value">{{ tunnel.custom_health_url }}</span>
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
  border: 1px solid var(--corvus-color-border-default);
  border-radius: var(--corvus-radius-input);
  background: var(--corvus-color-bg-surface);
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

.status-label {
  font-weight: 500;
  flex: 1;
}

.status-value {
  font-size: 12px;
  color: var(--corvus-color-text-secondary);
}
</style>
