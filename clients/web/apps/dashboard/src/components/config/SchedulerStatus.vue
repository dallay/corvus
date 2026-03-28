<script setup lang="ts">
import { validateGatewayUrl } from "@corvus/shared";
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { AdminSchedulerStatusView } from "@/types/admin-config";

const props = defineProps<{
  gatewayUrl: string;
  bearerToken: string;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();

const scheduler = ref<AdminSchedulerStatusView | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);

async function fetchSchedulerStatus() {
  loading.value = true;
  error.value = null;
  try {
    const base = validateGatewayUrl(props.gatewayUrl);
    if (!base) {
      throw new Error("Invalid gateway URL");
    }
    const baseStr = base.toString().replace(/\/+$/, "");
    const requestUrl = new URL("web/admin/scheduler", `${baseStr}/`);
    const res = await fetch(requestUrl.toString(), {
      headers: { Authorization: `Bearer ${props.bearerToken}` },
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json();
    scheduler.value = data.scheduler ?? null;
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

watch(() => [props.gatewayUrl, props.bearerToken], fetchSchedulerStatus, { immediate: true });
</script>

<template>
  <section class="card">
    <h2>{{ t("sections.schedulerStatus") }}</h2>
    <p v-if="loading" class="helper">{{ t("scheduler.loading") }}</p>
    <p v-else-if="error" class="error">{{ error }}</p>
    <div v-else-if="scheduler" class="status-grid" data-testid="scheduler-status">
      <div class="status-item">
        <span class="status-label">{{ t("scheduler.enabled") }}</span>
        <span
          class="status-indicator"
          :class="scheduler.enabled ? 'configured' : 'not-configured'"
        />
        <span class="status-value">{{
          scheduler.enabled ? t("scheduler.yes") : t("scheduler.no")
        }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("scheduler.maxTasks") }}</span>
        <span class="status-value">{{ scheduler.max_tasks }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("scheduler.maxConcurrent") }}</span>
        <span class="status-value">{{ scheduler.max_concurrent }}</span>
      </div>
      <div class="status-item">
        <span class="status-label">{{ t("scheduler.taskCount") }}</span>
        <span class="status-value">{{ scheduler.task_count }}</span>
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
