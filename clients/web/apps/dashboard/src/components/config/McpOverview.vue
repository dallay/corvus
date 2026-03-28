<script setup lang="ts">
import { validateGatewayUrl } from "@corvus/shared";
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { AdminMcpView } from "@/types/admin-config";

const props = defineProps<{
  gatewayUrl: string;
  bearerToken: string;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();

const mcp = ref<AdminMcpView | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);

async function fetchMcp() {
  loading.value = true;
  error.value = null;
  try {
    const base = validateGatewayUrl(props.gatewayUrl);
    if (!base) {
      mcp.value = null;
      error.value = "Invalid gateway URL";
      return;
    }

    const baseStr = base.toString().replace(/\/+$/, "");
    const requestUrl = new URL("web/admin/config", `${baseStr}/`);
    const res = await fetch(requestUrl.toString(), {
      headers: { Authorization: `Bearer ${props.bearerToken}` },
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json();
    mcp.value = data.config?.mcp ?? null;
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

watch(() => [props.gatewayUrl, props.bearerToken], fetchMcp, { immediate: true });
</script>

<template>
  <section class="card">
    <h2>{{ t("sections.mcp") }}</h2>
    <p v-if="loading" class="helper">{{ t("mcp.loading") }}</p>
    <p v-else-if="error" class="error">{{ error }}</p>
    <template v-else-if="mcp">
      <div class="status-grid">
        <div class="status-item">
          <span class="status-label">{{ t("mcp.enabled") }}</span>
          <span
            class="status-indicator"
            :class="mcp.enabled ? 'configured' : 'not-configured'"
          />
          <span class="status-value">{{
            mcp.enabled ? t("mcp.yes") : t("mcp.no")
          }}</span>
        </div>
      </div>
      <div v-if="Array.isArray(mcp.servers) && mcp.servers.length > 0" class="server-list">
        <div
          v-for="server in mcp.servers"
          :key="server.name"
          class="server-item"
          :data-testid="'mcp-server-' + server.name"
        >
          <div class="server-header">
            <span
              class="status-indicator"
              :class="server.enabled ? 'configured' : 'not-configured'"
            />
            <span class="server-name">{{ server.name }}</span>
          </div>
          <div class="server-details">
            <span class="detail-label">{{ t("mcp.command") }}:</span>
            <code class="detail-value">{{ server.command }}</code>
          </div>
          <div v-if="Array.isArray(server.capabilities) && server.capabilities.length > 0" class="server-details">
            <span class="detail-label">{{ t("mcp.capabilities") }}:</span>
            <span class="detail-value">{{ server.capabilities.join(", ") }}</span>
          </div>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.status-grid {
  display: grid;
  gap: 8px;
  margin-bottom: 12px;
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

.server-list {
  display: grid;
  gap: 8px;
}

.server-item {
  padding: 10px 12px;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-bg-secondary) 82%, transparent);
}

.server-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}

.server-name {
  font-weight: 500;
}

.server-details {
  display: flex;
  gap: 6px;
  font-size: 12px;
  color: var(--color-text-secondary);
  margin-top: 4px;
}

.detail-label {
  flex-shrink: 0;
}

.detail-value {
  word-break: break-all;
}
</style>
