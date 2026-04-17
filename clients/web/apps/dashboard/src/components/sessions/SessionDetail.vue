<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import CerebroSessionActions from "@/components/sessions/CerebroSessionActions.vue";
import { useAdmin } from "@/composables/useAdmin";

const props = defineProps<{
  gatewayUrl: (path: string) => string;
  authHeaders: () => Record<string, string>;
  sessionId: string;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const emit = defineEmits<{
  (e: "view-memory", sessionId: string): void;
  (e: "close"): void;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();
const admin = useAdmin(props.gatewayUrl, props.authHeaders);
const closeButtonRef = ref<HTMLButtonElement | null>(null);

function focusCloseButton(): void {
  closeButtonRef.value?.focus();
}

defineExpose({
  focusCloseButton,
});

async function load() {
  const [sessionResult, cerebroResult] = await Promise.allSettled([
    admin.fetchSessionDetail(props.sessionId),
    admin.fetchCerebroStatus(),
  ]);

  if (sessionResult.status === "rejected") {
    throw sessionResult.reason;
  }

  if (cerebroResult.status === "rejected") {
    console.error("Failed to fetch Cerebro status", cerebroResult.reason);
  }
}

watch(() => props.sessionId, load, { immediate: true });
</script>

<template>
  <div class="session-detail">
    <div class="detail-header">
      <h3>{{ t("sessions.detail", "Session Detail") }}</h3>
      <button ref="closeButtonRef" class="close-btn touch-target" :aria-label="t('actions.close', 'Close')" @click="emit('close')">
        &times;
      </button>
    </div>

    <p v-if="admin.loadingBuckets.value.sessionDetail" class="helper" aria-live="polite">
      {{ t("sessions.loading", "Loading…") }}
    </p>
    <p v-else-if="admin.error.value" class="error" aria-live="assertive">
      {{ admin.error.value }}
    </p>
    <template v-else-if="admin.sessionDetail.value">
      <dl class="detail-grid">
        <div class="detail-item">
          <dt>{{ t("sessions.colId", "Session ID") }}</dt>
          <dd class="mono">{{ admin.sessionDetail.value.id }}</dd>
        </div>
        <div class="detail-item">
          <dt>{{ t("sessions.colStatus", "Status") }}</dt>
          <dd>
            <span
              class="status-badge"
              :class="admin.sessionDetail.value.status === 'active' ? 'status-active' : 'status-ended'"
            >
              {{ admin.sessionDetail.value.status === "active"
                ? t("sessions.statusActive", "Active")
                : t("sessions.statusEnded", "Ended") }}
            </span>
          </dd>
        </div>
        <div class="detail-item">
          <dt>{{ t("sessions.colStarted", "Started") }}</dt>
          <dd>{{ admin.sessionDetail.value.started_at }}</dd>
        </div>
        <div class="detail-item">
          <dt>{{ t("sessions.endedAt", "Ended At") }}</dt>
          <dd>{{ admin.sessionDetail.value.ended_at ?? t("sessions.statusActive", "Active") }}</dd>
        </div>
        <div class="detail-item">
          <dt>{{ t("sessions.colMessages", "Messages") }}</dt>
          <dd>{{ admin.sessionDetail.value.message_count }}</dd>
        </div>
        <div class="detail-item">
          <dt>{{ t("sessions.colLastActivity", "Last Activity") }}</dt>
          <dd>{{ admin.sessionDetail.value.last_activity }}</dd>
        </div>
      </dl>

      <div class="memory-summary">
        <h4>{{ t("sessions.memorySummary", "Memory Summary") }}</h4>
        <template v-if="Object.keys(admin.sessionDetail.value.memory_summary).length > 0">
          <div
            v-for="(count, category) in admin.sessionDetail.value.memory_summary"
            :key="String(category)"
            class="summary-row"
          >
            <span class="category-name">{{ category }}</span>
            <span class="category-count">{{ count }}</span>
          </div>
        </template>
        <p v-else class="helper">
          {{ t("sessions.noMemoryEntries", "0 entries") }}
        </p>
      </div>

      <button class="view-memory-btn" @click="emit('view-memory', admin.sessionDetail.value.id)">
        {{ t("sessions.viewMemory", "View Memory Entries") }}
      </button>

      <CerebroSessionActions
        :gateway-url="gatewayUrl"
        :auth-headers="authHeaders"
        :session-id="sessionId"
        :status="admin.cerebroStatus.value"
      />
    </template>
  </div>
</template>

<style scoped>
.session-detail {
  border: 1px solid var(--color-border);
  border-radius: 14px;
  padding: 16px;
  background: color-mix(in srgb, var(--color-bg-secondary) 82%, transparent);
}

.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.detail-header h3 {
  margin: 0;
  font-size: 14px;
}

.touch-target {
  min-height: 24px;
  min-width: 24px;
}

.close-btn {
  background: none;
  border: none;
  font-size: 20px;
  cursor: pointer;
  color: var(--color-text-secondary);
  padding: 6px;
  line-height: 1;
}

.detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  margin: 0;
}

.detail-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.detail-item dt {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--color-text-secondary);
}

.detail-item dd {
  margin: 0;
  font-size: 13px;
}

.mono {
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
}

.status-badge {
  display: inline-block;
  border-radius: 999px;
  padding: 2px 8px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.status-active {
  background: color-mix(in srgb, #22c55e 15%, var(--color-bg-secondary));
  color: #22c55e;
}

.status-ended {
  background: color-mix(in srgb, #9ca3af 15%, var(--color-bg-secondary));
  color: #9ca3af;
}

.memory-summary {
  margin-top: 16px;
}

.memory-summary h4 {
  margin: 0 0 8px;
  font-size: 13px;
}

.summary-row {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
  font-size: 13px;
  border-bottom: 1px solid var(--color-border);
}

.category-name {
  text-transform: capitalize;
}

.category-count {
  font-weight: 600;
  color: var(--color-text-secondary);
}

.view-memory-btn {
  margin-top: 12px;
  padding: 6px 14px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  cursor: pointer;
  font-size: 12px;
}

.view-memory-btn:hover {
  background: color-mix(in srgb, var(--color-bg-secondary) 60%, transparent);
}
</style>
