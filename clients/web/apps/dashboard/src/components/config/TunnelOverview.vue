<script setup lang="ts">
import { trimTrailingSlashes } from "@corvus/shared";
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { AdminTunnelView } from "@/types/admin-config";

const props = defineProps<{
  gatewayUrl: string;
  bearerToken: string;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();

const tunnel = ref<AdminTunnelView | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);

async function fetchTunnel() {
  loading.value = true;
  error.value = null;
  try {
    const base = trimTrailingSlashes(props.gatewayUrl);
    const res = await fetch(`${base}/web/admin/config`, {
      headers: { Authorization: `Bearer ${props.bearerToken}` },
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json();
    tunnel.value = data.config?.tunnel ?? null;
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(fetchTunnel);
</script>

<template>
  <section class="card">
    <h2>{{ t("sections.tunnel") }}</h2>
    <p v-if="loading" class="helper">{{ t("tunnel.loading") }}</p>
    <p v-else-if="error" class="error">{{ error }}</p>
    <div v-else-if="tunnel" class="status-grid" data-testid="tunnel-overview">
      <div class="status-item">
        <span class="status-label">{{ t("tunnel.provider") }}</span>
        <span class="status-value">{{ tunnel.provider }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("tunnel.cloudflareToken") }}</span>
        <span
          class="status-indicator"
          :class="tunnel.has_cloudflare_token ? 'configured' : 'not-configured'"
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
