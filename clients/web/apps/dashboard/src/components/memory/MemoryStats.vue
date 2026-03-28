<script setup lang="ts">
import { onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { useAdmin } from "@/composables/useAdmin";

const props = defineProps<{
  gatewayUrl: (path: string) => string;
  authHeaders: () => Record<string, string>;
}>();

const { t } = useI18n();
const admin = useAdmin(props.gatewayUrl, props.authHeaders);

onMounted(() => admin.fetchMemoryStats());
</script>

<template>
  <div class="memory-stats">
    <p v-if="admin.loading.value" class="helper" aria-live="polite" role="status">
      {{ t("memory.statsLoading", "Loading stats…") }}
    </p>
    <p v-else-if="admin.error.value" class="error" aria-live="assertive" role="alert">
      {{ admin.error.value }}
    </p>
    <template v-else-if="admin.memoryStats.value">
      <div class="stats-grid">
        <div class="stat-card">
          <span class="stat-value">{{ admin.memoryStats.value.total_entries }}</span>
          <span class="stat-label">{{ t("memory.statTotalEntries", "Total Entries") }}</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ admin.memoryStats.value.total_sessions }}</span>
          <span class="stat-label">{{ t("memory.statTotalSessions", "Total Sessions") }}</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ admin.memoryStats.value.active_sessions }}</span>
          <span class="stat-label">{{ t("memory.statActiveSessions", "Active Sessions") }}</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ admin.memoryStats.value.backend }}</span>
          <span class="stat-label">{{ t("memory.statBackend", "Backend") }}</span>
        </div>
        <div class="stat-card">
          <span
            class="stat-value"
            :class="admin.memoryStats.value.cerebro_configured ? 'indicator-ok' : 'indicator-off'"
          >
            {{ admin.memoryStats.value.cerebro_configured
              ? t("memory.cerebroConfigured", "Configured")
              : t("memory.cerebroNotConfigured", "Not configured") }}
          </span>
          <span class="stat-label">{{ t("memory.statCerebro", "Cerebro") }}</span>
        </div>
      </div>

      <div
        v-if="Object.keys(admin.memoryStats.value.by_category).length > 0"
        class="category-breakdown"
      >
        <h4>{{ t("memory.statByCategory", "By Category") }}</h4>
        <div class="category-grid">
          <div
            v-for="(count, cat) in admin.memoryStats.value.by_category"
            :key="String(cat)"
            class="category-item"
          >
            <span class="category-name">{{ cat }}</span>
            <span class="category-count">{{ count }}</span>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.stats-grid {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.stat-card {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 14px;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-bg-secondary) 82%, transparent);
  min-width: 100px;
}

.stat-value {
  font-size: 18px;
  font-weight: 600;
}

.stat-label {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--color-text-secondary);
}

.indicator-ok {
  color: #22c55e;
  font-size: 13px;
}

.indicator-off {
  color: #9ca3af;
  font-size: 13px;
}

.category-breakdown {
  margin-top: 12px;
}

.category-breakdown h4 {
  margin: 0 0 8px;
  font-size: 13px;
}

.category-grid {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.category-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  font-size: 13px;
}

.category-name {
  text-transform: capitalize;
}

.category-count {
  font-weight: 600;
  color: var(--color-text-secondary);
}
</style>
