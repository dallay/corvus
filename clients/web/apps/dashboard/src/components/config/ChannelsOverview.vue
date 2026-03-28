<script setup lang="ts">
import { trimTrailingSlashes, validateGatewayUrl } from "@corvus/shared";
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { AdminChannelStatusView } from "@/types/admin-config";

const props = defineProps<{
  gatewayUrl: string;
  bearerToken: string;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();

const channels = ref<AdminChannelStatusView[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);

async function fetchChannels() {
  loading.value = true;
  error.value = null;
  try {
    const base = validateGatewayUrl(props.gatewayUrl);
    if (!base) {
      throw new Error("Invalid gateway URL");
    }
    const baseStr = trimTrailingSlashes(base.toString());
    const requestUrl = new URL("web/admin/channels", `${baseStr}/`);
    const res = await fetch(requestUrl.toString(), {
      headers: { Authorization: `Bearer ${props.bearerToken}` },
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json();
    channels.value = data.channels ?? [];
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

watch(() => [props.gatewayUrl, props.bearerToken], fetchChannels, { immediate: true });
</script>

<template>
  <section class="card">
    <h2>{{ t("sections.channels") }}</h2>
    <p v-if="loading" class="helper" aria-live="polite" role="status">{{ t("channels.loading") }}</p>
    <p v-else-if="error" class="error" aria-live="assertive" role="alert">{{ error }}</p>
    <div v-else class="channel-list">
      <div
        v-for="ch in channels"
        :key="ch.channel_type"
        class="channel-item"
        :data-testid="'channel-' + ch.channel_type"
      >
        <span
          class="channel-indicator"
          :class="ch.configured ? 'configured' : 'not-configured'"
          aria-hidden="true"
        />
        <span class="channel-name">{{ ch.channel_type }}</span>
        <span class="channel-status">{{
          ch.configured ? t("channels.configured") : t("channels.notConfigured")
        }}</span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.channel-list {
  display: grid;
  gap: 8px;
}

.channel-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-bg-secondary) 82%, transparent);
}

.channel-indicator {
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

.channel-name {
  font-weight: 500;
  flex: 1;
}

.channel-status {
  font-size: 12px;
  color: var(--color-text-secondary);
}
</style>
