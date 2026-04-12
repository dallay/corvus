<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { type SessionListParams, useAdmin } from "@/composables/useAdmin";
import type { AdminSessionView } from "@/types/admin-sessions";

const props = defineProps<{
  gatewayUrl: (path: string) => string;
  authHeaders: () => Record<string, string>;
  statusFilter?: "active" | "ended";
  sort?: "last_activity" | "started_at";
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const emit = defineEmits<(e: "select", session: AdminSessionView) => void>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();
const admin = useAdmin(props.gatewayUrl, props.authHeaders);
const page = ref(1);
const perPage = ref(25);

const PER_PAGE_OPTIONS = [25, 50, 100] as const;
const PER_PAGE_OPTIONS_SET = new Set<number>(PER_PAGE_OPTIONS);

async function load() {
  const params: SessionListParams = {
    status: props.statusFilter,
    page: page.value,
    per_page: perPage.value,
    sort: props.sort ?? "last_activity",
    order: "desc",
  };
  await admin.fetchSessions(params);
}

function totalPages(): number {
  return Math.max(1, Math.ceil(admin.totalSessions.value / perPage.value));
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function goToPage(p: number) {
  if (p >= 1 && p <= totalPages()) {
    page.value = p;
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function onPerPageChange(event: Event) {
  const value = Number((event.target as HTMLSelectElement | null)?.value);
  perPage.value = PER_PAGE_OPTIONS_SET.has(value) ? value : 25;
  if (page.value === 1) {
    load();
  } else {
    page.value = 1;
  }
}

watch([() => props.statusFilter, () => props.sort], () => {
  if (page.value === 1) {
    load();
  } else {
    page.value = 1;
  }
});

watch(page, () => load());

onMounted(() => load());
</script>

<template>
  <div class="session-list">
    <p v-if="admin.loading.value" class="helper" aria-live="polite">
      {{ t("sessions.loading", "Loading sessions…") }}
    </p>
    <p v-else-if="admin.error.value" class="error" aria-live="assertive">
      {{ admin.error.value }}
    </p>
    <template v-else>
      <p v-if="admin.sessions.value.length === 0" class="helper">
        {{ t("sessions.empty", "No sessions found") }}
      </p>
      <table v-else class="session-table" aria-label="Sessions">
        <thead>
          <tr>
            <th>{{ t("sessions.colId", "Session ID") }}</th>
            <th>{{ t("sessions.colStarted", "Started") }}</th>
            <th>{{ t("sessions.colLastActivity", "Last Activity") }}</th>
            <th>{{ t("sessions.colMessages", "Messages") }}</th>
            <th>{{ t("sessions.colStatus", "Status") }}</th>
            <th><span class="sr-only">{{ t("sessions.colActions", "Actions") }}</span></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="session in admin.sessions.value"
            :key="session.id"
            class="session-row"
            :data-testid="'session-' + session.id"
          >
            <td class="session-id">{{ session.id }}</td>
            <td>{{ session.started_at }}</td>
            <td>{{ session.last_activity }}</td>
            <td>{{ session.message_count }}</td>
            <td>
              <span
                class="status-badge"
                :class="session.status === 'active' ? 'status-active' : 'status-ended'"
              >
                {{ session.status === "active"
                  ? t("sessions.statusActive", "Active")
                  : t("sessions.statusEnded", "Ended") }}
              </span>
            </td>
            <td class="action-cell">
              <button
                class="select-btn touch-target"
                :data-testid="'view-session-' + session.id"
                :aria-label="t('sessions.selectSession', 'View session') + ' ' + session.id"
                @click="emit('select', session)"
              >
                {{ t("sessions.view", "View") }}
              </button>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-if="totalPages() > 1" class="pagination">
        <label class="page-size-control">
          <span>{{ t("pagination.perPage", "Rows per page") }}</span>
          <select :value="String(perPage)" class="page-size-select touch-target" @change="onPerPageChange">
            <option v-for="option in PER_PAGE_OPTIONS" :key="option" :value="option">
              {{ option }}
            </option>
          </select>
        </label>
        <button :disabled="page <= 1" class="touch-target" @click="goToPage(page - 1)">
          {{ t("pagination.prev", "Previous") }}
        </button>
        <span class="page-info">
          {{ t("pagination.page", "Page") }} {{ page }} / {{ totalPages() }}
          ({{ admin.totalSessions.value }} {{ t("pagination.total", "total") }})
        </span>
        <button :disabled="page >= totalPages()" class="touch-target" @click="goToPage(page + 1)">
          {{ t("pagination.next", "Next") }}
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.session-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.session-table th {
  text-align: left;
  padding: 8px 10px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--color-text-secondary);
  border-bottom: 1px solid var(--color-border);
}

.session-table td {
  padding: 8px 10px;
  border-bottom: 1px solid var(--color-border);
}

.session-row {
  transition: background 0.15s;
}

.session-row:hover {
  background: color-mix(in srgb, var(--color-bg-secondary) 60%, transparent);
}

.session-id {
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.touch-target {
  min-height: 24px;
  min-width: 24px;
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

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 12px;
  font-size: 13px;
}

.page-size-control {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  color: var(--color-text-secondary);
  font-size: 12px;
}

.page-size-select {
  min-width: 72px;
  height: 32px;
  border-radius: 8px;
  border: 1px solid var(--color-border);
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  font-family: inherit;
  font-size: 12px;
  padding: 0 8px;
}

.pagination button {
  padding: 4px 12px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  cursor: pointer;
  font-size: 12px;
  min-height: 32px;
  min-width: 32px;
}

.select-btn {
  min-height: 32px;
  min-width: 32px;
  padding: 6px 12px;
}

.pagination button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.page-info {
  color: var(--color-text-secondary);
  font-size: 12px;
}

.action-cell {
  text-align: right;
}

.select-btn {
  padding: 2px 10px;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  cursor: pointer;
  font-size: 11px;
}

.select-btn:hover {
  background: color-mix(in srgb, var(--color-bg-secondary) 60%, transparent);
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
</style>
