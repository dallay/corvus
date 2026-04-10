<script setup lang="ts">
import { onMounted } from "vue";
import { useI18n } from "vue-i18n";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import CerebroStatusCard from "@/components/memory/CerebroStatusCard.vue";
import { useAdmin } from "@/composables/useAdmin";

const props = defineProps<{
  gatewayUrl: (path: string) => string;
  authHeaders: () => Record<string, string>;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const emit = defineEmits<{
  "select-category": [category: string];
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();
const admin = useAdmin(props.gatewayUrl, props.authHeaders);

onMounted(async () => {
  await admin.fetchMemoryStats();

  await Promise.allSettled([admin.fetchCerebroStatus(), admin.fetchCerebroStats()]);
});
</script>

<template>
  <div class="memory-stats">
    <p v-if="admin.loadingBuckets.value.memoryStats" class="helper" aria-live="polite">
      {{ t("memory.statsLoading", "Loading stats…") }}
    </p>
    <p v-else-if="admin.error.value && !admin.memoryStats.value" class="error" aria-live="assertive">
      {{ admin.error.value }}
    </p>
    <template v-else-if="admin.memoryStats.value">
      <h3 class="section-title">Local Memory</h3>
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
      </div>

      <div class="remote-grid">
        <CerebroStatusCard :status="admin.cerebroStatus.value" />

        <section class="cerebro-remote-card">
          <header class="card-header">
            <div>
              <p class="eyebrow">Cerebro Memory</p>
              <h4>Remote Stats</h4>
            </div>
          </header>
          <p
            v-if="admin.cerebroStats.value && 'state' in admin.cerebroStats.value && admin.cerebroStats.value.state !== 'available'"
            class="helper"
          >
            {{ admin.cerebroStats.value.message }}
          </p>
          <div v-else-if="admin.cerebroStats.value && 'stats' in admin.cerebroStats.value" class="stats-grid">
            <div class="stat-card">
              <span class="stat-value">{{ admin.cerebroStats.value.stats.memory_count }}</span>
              <span class="stat-label">Remote Memories</span>
            </div>
            <div class="stat-card">
              <span class="stat-value">{{ admin.cerebroStats.value.stats.session_count }}</span>
              <span class="stat-label">Remote Sessions</span>
            </div>
            <div class="stat-card">
              <span class="stat-value">{{ admin.cerebroStats.value.stats.prompt_count }}</span>
              <span class="stat-label">Saved Prompts</span>
            </div>
            <div class="stat-card">
              <span class="stat-value">{{ admin.cerebroStats.value.stats.worker_queue_depth }}</span>
              <span class="stat-label">Worker Queue</span>
            </div>
          </div>
        </section>
      </div>

      <div v-if="Object.keys(admin.memoryStats.value.by_category).length > 0" class="category-breakdown">
        <h4>{{ t("memory.statByCategory", "By Category") }}</h4>
        <div class="category-grid">
          <button
            v-for="(count, cat) in admin.memoryStats.value.by_category"
            :key="String(cat)"
            type="button"
            class="category-item"
            @click="emit('select-category', String(cat))"
          >
            <span class="category-name">{{ cat }}</span>
            <span class="category-count">{{ count }}</span>
          </button>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.section-title {
  margin: 0 0 10px;
  font-size: 14px;
}

.stats-grid {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.remote-grid {
  display: grid;
  gap: 12px;
  margin-top: 14px;
}

.stat-card,
.cerebro-remote-card {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 14px;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-bg-secondary) 82%, transparent);
  min-width: 100px;
}

.card-header {
  margin-bottom: 8px;
}

.eyebrow {
  margin: 0 0 4px;
  font-size: 11px;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.card-header h4 {
  margin: 0;
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
  background: transparent;
  color: inherit;
  cursor: pointer;
}

.category-name {
  text-transform: capitalize;
}

.category-count {
  font-weight: 600;
  color: var(--color-text-secondary);
}
</style>
