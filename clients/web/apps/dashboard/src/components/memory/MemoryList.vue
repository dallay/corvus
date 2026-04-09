<script lang="ts" setup>
import { nextTick, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { type MemoryListParams, useAdmin } from "@/composables/useAdmin";
import type { LocalMemoryExplorerSelection } from "@/types/admin-sessions";

const props = defineProps<{
  gatewayUrl: (path: string) => string;
  authHeaders: () => Record<string, string>;
  categoryFilter?: string;
  sessionIdFilter?: string;
  searchFilter?: string;
}>();

const emit = defineEmits<{
  "select-category": [category: string];
  "select-session": [sessionId?: string];
  "open-explorer": [selection: LocalMemoryExplorerSelection];
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();
const admin = useAdmin(props.gatewayUrl, props.authHeaders);
const page = ref(1);
const perPage = ref(25);
const confirmingDelete = ref<string | null>(null);
const confirmBtnRef = ref<HTMLButtonElement | null>(null);
const restoreFocusTarget = ref<HTMLElement | null>(null);

async function load() {
  const params: MemoryListParams = {
    category: props.categoryFilter,
    session_id: props.sessionIdFilter,
    search: props.searchFilter,
    page: page.value,
    per_page: perPage.value,
  };
  await admin.fetchMemoryEntries(params);
}

function totalPages(): number {
  return Math.max(1, Math.ceil(admin.totalMemoryEntries.value / perPage.value));
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function goToPage(p: number) {
  if (p >= 1 && p <= totalPages()) {
    page.value = p;
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function requestDelete(key: string, event?: Event) {
  restoreFocusTarget.value =
    event?.currentTarget instanceof HTMLElement ? event.currentTarget : null;
  confirmingDelete.value = key;
  nextTick(() => confirmBtnRef.value?.focus());
}

function closeDeleteDialog() {
  confirmingDelete.value = null;
  nextTick(() => restoreFocusTarget.value?.focus());
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function cancelDelete() {
  closeDeleteDialog();
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function confirmDelete() {
  if (!confirmingDelete.value) return;
  const deleted = await admin.deleteMemoryEntry(confirmingDelete.value);
  closeDeleteDialog();
  if (deleted) {
    await load();
    page.value = Math.min(page.value, totalPages());
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function onSelectSession(sessionId?: string | null) {
  if (!sessionId) {
    return;
  }

  emit("select-session", sessionId);
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function truncate(text: string, maxLen: number): string {
  return text.length > maxLen ? `${text.slice(0, maxLen)}…` : text;
}

watch([() => props.categoryFilter, () => props.sessionIdFilter, () => props.searchFilter], () => {
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
  <div class="memory-list">
    <header class="memory-list-header">
      <div>
        <p class="memory-kicker">{{ t("memory.kicker", "Local Memory") }}</p>
        <h3>{{ t("memory.title", "SQLite-backed memory browser") }}</h3>
      </div>
      <p class="helper">
        {{
          t(
              "memory.helper",
              "Local memory stays available even when Cerebro is unconfigured, unreachable, or partially implemented."
          )
        }}
      </p>
    </header>
    <p v-if="admin.loading.value" aria-live="polite" class="helper" role="status">
      {{ t("memory.loading", "Loading memory entries…") }}
    </p>
    <p v-else-if="admin.error.value" aria-live="assertive" class="error" role="alert">
      {{ admin.error.value }}
    </p>
    <template v-else>
      <p v-if="admin.memoryEntries.value.length === 0" class="helper">
        {{ t("memory.empty", "No memory entries found") }}
      </p>
      <table v-else aria-label="Memory entries" class="memory-table">
        <thead>
        <tr>
          <th>{{ t("memory.colKey", "Key") }}</th>
          <th>{{ t("memory.colCategory", "Category") }}</th>
          <th>{{ t("memory.colTimestamp", "Timestamp") }}</th>
          <th>{{ t("memory.colSessionId", "Session ID") }}</th>
          <th>{{ t("memory.colContent", "Content") }}</th>
          <th>{{ t("memory.colActions", "Actions") }}</th>
        </tr>
        </thead>
        <tbody>
        <tr
            v-for="entry in admin.memoryEntries.value"
            :key="entry.id"
            :data-testid="'memory-' + entry.key"
        >
          <td class="mono">{{ entry.key }}</td>
          <td>
            <button class="category-badge" type="button"
                    @click="emit('select-category', entry.category)">
              {{ entry.category }}
            </button>
          </td>
          <td>{{ entry.timestamp }}</td>
          <td class="mono">
            <button
                :aria-label="entry.session_id ? `Filter by session ${entry.session_id}` : 'No session'"
                :disabled="!entry.session_id"
                class="session-link"
                type="button"
                @click="onSelectSession(entry.session_id)"
            >
              {{ entry.session_id ?? "No Session" }}
            </button>
          </td>
          <td class="content-cell">{{ truncate(entry.content, 80) }}</td>
          <td>
            <button
                class="explore-btn"
                type="button"
                @click="emit('open-explorer', { category: entry.category, sessionId: entry.session_id ?? undefined, entryId: entry.id })"
            >
              Explore
            </button>
            <button
                :aria-label="t('memory.delete', 'Delete') + ' ' + entry.key"
                class="delete-btn"
                @click="requestDelete(entry.key, $event)"
            >
              {{ t("memory.delete", "Delete") }}
            </button>
          </td>
        </tr>
        </tbody>
      </table>

      <div v-if="totalPages() > 1" class="pagination">
        <button :disabled="page <= 1" @click="goToPage(page - 1)">
          {{ t("pagination.prev", "Previous") }}
        </button>
        <span class="page-info">
          {{ t("pagination.page", "Page") }} {{ page }} / {{ totalPages() }}
          ({{ admin.totalMemoryEntries.value }} {{ t("pagination.total", "total") }})
        </span>
        <button :disabled="page >= totalPages()" @click="goToPage(page + 1)">
          {{ t("pagination.next", "Next") }}
        </button>
      </div>
    </template>

    <!-- Delete confirmation dialog -->
    <div
        v-if="confirmingDelete"
        class="confirm-overlay"
        @click.self="cancelDelete"
        @keydown.escape="cancelDelete"
    >
      <div
          aria-describedby="memory-delete-description"
          aria-labelledby="memory-delete-title"
          aria-modal="true"
          class="confirm-dialog"
          role="alertdialog"
      >
        <h2 id="memory-delete-title" class="confirm-title">
          {{ t("memory.confirmDelete", "Delete memory entry") }}
        </h2>
        <p id="memory-delete-description">
          {{ t("memory.confirmDeletePrompt", "Delete memory entry") }}
          <strong class="mono">{{ confirmingDelete }}</strong>?
        </p>
        <div class="confirm-actions">
          <button ref="confirmBtnRef" class="confirm-btn confirm-yes" @click="confirmDelete">
            {{ t("memory.confirmYes", "Delete") }}
          </button>
          <button class="confirm-btn confirm-no" @click="cancelDelete">
            {{ t("memory.confirmNo", "Cancel") }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.memory-table {
  border-collapse: collapse;
  font-size: 13px;
  width: 100%;
}

.memory-table th {
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text-secondary);
  font-size: 11px;
  letter-spacing: 0.04em;
  padding: 8px 10px;
  text-align: left;
  text-transform: uppercase;
}

.memory-table td {
  border-bottom: 1px solid var(--color-border);
  padding: 8px 10px;
}

.mono {
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
}

.content-cell {
  color: var(--color-text-secondary);
  max-width: 240px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.category-badge {
  background: color-mix(in srgb, var(--color-bg-input) 80%, transparent);
  border: 1px solid var(--color-border);
  border-radius: 999px;
  color: inherit;
  cursor: pointer;
  display: inline-block;
  font-size: 11px;
  font-weight: 500;
  padding: 2px 8px;
  text-transform: capitalize;
}

.session-link,
.explore-btn {
  background: transparent;
  border: 0;
  color: inherit;
  cursor: pointer;
  padding: 0;
}

.explore-btn {
  color: var(--color-primary, var(--color-text-primary));
  margin-right: 8px;
}

.session-link:disabled {
  cursor: not-allowed;
  opacity: 0.7;
}

.delete-btn {
  background: transparent;
  border: 1px solid color-mix(in srgb, #ef4444 40%, var(--color-border));
  border-radius: 6px;
  color: #ef4444;
  cursor: pointer;
  font-size: 11px;
  padding: 2px 8px;
}

.delete-btn:hover {
  background: color-mix(in srgb, #ef4444 10%, var(--color-bg-secondary));
}

.pagination {
  align-items: center;
  display: flex;
  font-size: 13px;
  gap: 12px;
  justify-content: center;
  margin-top: 12px;
}

.pagination button {
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  color: var(--color-text-primary);
  cursor: pointer;
  font-size: 12px;
  padding: 4px 12px;
}

.pagination button:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}

.page-info {
  color: var(--color-text-secondary);
  font-size: 12px;
}

.confirm-overlay {
  align-items: center;
  background: rgb(0 0 0 / 0.4);
  display: flex;
  inset: 0;
  justify-content: center;
  position: fixed;
  z-index: 100;
}

.confirm-dialog {
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: 14px;
  max-width: 400px;
  padding: 20px;
  width: 90%;
}

.confirm-dialog p {
  font-size: 14px;
  margin: 0 0 16px;
}

.confirm-title {
  font-size: 16px;
  margin: 0 0 8px;
}

.confirm-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.confirm-btn {
  border: 1px solid var(--color-border);
  border-radius: 8px;
  cursor: pointer;
  font-size: 12px;
  padding: 6px 14px;
}

.confirm-yes {
  background: #ef4444;
  border-color: #ef4444;
  color: #fff;
}

.confirm-no {
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
}

.memory-list-header {
  align-items: flex-start;
  display: flex;
  gap: 12px;
  justify-content: space-between;
  margin-bottom: 12px;
}

.memory-list-header h3,
.memory-kicker {
  margin: 0;
}

.memory-kicker {
  color: var(--color-text-secondary);
  font-size: 11px;
  text-transform: uppercase;
}

</style>
