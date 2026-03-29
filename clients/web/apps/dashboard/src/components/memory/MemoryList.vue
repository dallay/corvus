<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { type MemoryListParams, useAdmin } from "@/composables/useAdmin";

const props = defineProps<{
  gatewayUrl: (path: string) => string;
  authHeaders: () => Record<string, string>;
  categoryFilter?: string;
  sessionIdFilter?: string;
  searchFilter?: string;
}>();

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

function goToPage(p: number) {
  if (p >= 1 && p <= totalPages()) {
    page.value = p;
  }
}

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

function cancelDelete() {
  closeDeleteDialog();
}

async function confirmDelete() {
  if (!confirmingDelete.value) return;
  const deleted = await admin.deleteMemoryEntry(confirmingDelete.value);
  closeDeleteDialog();
  if (deleted) {
    await load();
    page.value = Math.min(page.value, totalPages());
  }
}

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
    <p v-if="admin.loading.value" class="helper" aria-live="polite" role="status">
      {{ t("memory.loading", "Loading memory entries…") }}
    </p>
    <p v-else-if="admin.error.value" class="error" aria-live="assertive" role="alert">
      {{ admin.error.value }}
    </p>
    <template v-else>
      <p v-if="admin.memoryEntries.value.length === 0" class="helper">
        {{ t("memory.empty", "No memory entries found") }}
      </p>
      <table v-else class="memory-table" aria-label="Memory entries">
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
              <span class="category-badge">{{ entry.category }}</span>
            </td>
            <td>{{ entry.timestamp }}</td>
            <td class="mono">{{ entry.session_id ?? "—" }}</td>
            <td class="content-cell">{{ truncate(entry.content, 80) }}</td>
            <td>
              <button
                class="delete-btn"
                :aria-label="t('memory.delete', 'Delete') + ' ' + entry.key"
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
        class="confirm-dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="memory-delete-title"
        aria-describedby="memory-delete-description"
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
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.memory-table th {
  text-align: left;
  padding: 8px 10px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--color-text-secondary);
  border-bottom: 1px solid var(--color-border);
}

.memory-table td {
  padding: 8px 10px;
  border-bottom: 1px solid var(--color-border);
}

.mono {
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
}

.content-cell {
  max-width: 240px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text-secondary);
}

.category-badge {
  display: inline-block;
  border-radius: 999px;
  padding: 2px 8px;
  font-size: 11px;
  font-weight: 500;
  text-transform: capitalize;
  background: color-mix(in srgb, var(--color-bg-input) 80%, transparent);
  border: 1px solid var(--color-border);
}

.delete-btn {
  padding: 2px 8px;
  border: 1px solid color-mix(in srgb, #ef4444 40%, var(--color-border));
  border-radius: 6px;
  background: transparent;
  color: #ef4444;
  cursor: pointer;
  font-size: 11px;
}

.delete-btn:hover {
  background: color-mix(in srgb, #ef4444 10%, var(--color-bg-secondary));
}

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-top: 12px;
  font-size: 13px;
}

.pagination button {
  padding: 4px 12px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
  cursor: pointer;
  font-size: 12px;
}

.pagination button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.page-info {
  color: var(--color-text-secondary);
  font-size: 12px;
}

.confirm-overlay {
  position: fixed;
  inset: 0;
  background: rgb(0 0 0 / 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.confirm-dialog {
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: 14px;
  padding: 20px;
  max-width: 400px;
  width: 90%;
}

.confirm-dialog p {
  margin: 0 0 16px;
  font-size: 14px;
}

.confirm-title {
  margin: 0 0 8px;
  font-size: 16px;
}

.confirm-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.confirm-btn {
  padding: 6px 14px;
  border-radius: 8px;
  border: 1px solid var(--color-border);
  cursor: pointer;
  font-size: 12px;
}

.confirm-yes {
  background: #ef4444;
  color: #fff;
  border-color: #ef4444;
}

.confirm-no {
  background: var(--color-bg-secondary);
  color: var(--color-text-primary);
}
</style>
