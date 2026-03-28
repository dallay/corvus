<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { AdminHealthSnapshot } from "@/types/admin-config";

const props = defineProps<{
  gatewayUrl: string;
  bearerToken: string;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();

const health = ref<AdminHealthSnapshot | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const overallStatus = computed(() => {
  if (!health.value) return "unknown";
  const statuses = Object.values(health.value.components);
  if (statuses.length === 0) return "ok";
  return statuses.every((c) => c.status === "ok") ? "ok" : "error";
});

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const parts: string[] = [];
  if (days > 0) parts.push(`${days}d`);
  if (hours > 0) parts.push(`${hours}h`);
  parts.push(`${minutes}m`);
  return parts.join(" ");
}

async function fetchHealth() {
  loading.value = true;
  error.value = null;
  try {
    const base = props.gatewayUrl.replace(/\/+$/, "");
    const res = await fetch(`${base}/web/admin/health`, {
      headers: { Authorization: `Bearer ${props.bearerToken}` },
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json();
    health.value = data.health ?? null;
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(fetchHealth);
</script>

<template>
  <section class="card">
    <h2>{{ t("sections.health") }}</h2>
    <p v-if="loading" class="helper">{{ t("health.loading") }}</p>
    <p v-else-if="error" class="error">{{ error }}</p>
    <template v-else-if="health">
      <div class="health-summary">
        <span class="health-indicator" :class="overallStatus" />
        <span>{{ t("health.uptime") }}: {{ formatUptime(health.uptime_seconds) }}</span>
      </div>
      <div class="component-list">
        <div
          v-for="(comp, name) in health.components"
          :key="name"
          class="component-item"
          :data-testid="'health-' + name"
        >
          <span
            class="component-indicator"
            :class="comp.status === 'ok' ? 'ok' : 'error'"
          />
          <span class="component-name">{{ name }}</span>
          <span class="component-status">{{ comp.status }}</span>
          <span v-if="comp.restart_count > 0" class="component-restarts">
            {{ t("health.restarts") }}: {{ comp.restart_count }}
          </span>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.health-summary {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
}

.health-indicator,
.component-indicator {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.health-indicator.ok,
.component-indicator.ok {
  background: #22c55e;
}

.health-indicator.error,
.component-indicator.error {
  background: #ef4444;
}

.health-indicator.unknown {
  background: #9ca3af;
}

.component-list {
  display: grid;
  gap: 8px;
}

.component-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-bg-secondary) 82%, transparent);
}

.component-name {
  font-weight: 500;
  flex: 1;
}

.component-status {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.component-restarts {
  font-size: 12px;
  color: var(--color-text-secondary);
}
</style>
