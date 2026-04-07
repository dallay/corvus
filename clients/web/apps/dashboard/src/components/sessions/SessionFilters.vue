<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";

const emit = defineEmits<{
  (e: "update:status", value: "active" | "ended" | undefined): void;
  (e: "update:sort", value: "last_activity" | "started_at"): void;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const { t } = useI18n();
const status = ref<string>("all");
const sort = ref<"last_activity" | "started_at">("last_activity");

const VALID_STATUSES = new Set(["all", "active", "ended"]);
const VALID_SORTS = new Set(["last_activity", "started_at"]);

function sanitizeStatus(value: string): "all" | "active" | "ended" {
  return VALID_STATUSES.has(value) ? (value as "all" | "active" | "ended") : "all";
}

function sanitizeSort(value: string): "last_activity" | "started_at" {
  return VALID_SORTS.has(value) ? (value as "last_activity" | "started_at") : "last_activity";
}

function onStatusChange(event?: Event) {
  status.value = sanitizeStatus(status.value);
  const target = event?.target as HTMLSelectElement | null;
  if (target) {
    target.value = status.value;
  }
  const value = status.value === "all" ? undefined : status.value;
  emit("update:status", value);
}

function onSortChange(event?: Event) {
  sort.value = sanitizeSort(sort.value);
  const target = event?.target as HTMLSelectElement | null;
  if (target) {
    target.value = sort.value;
  }
  emit("update:sort", sort.value);
}

onMounted(() => {
  onStatusChange();
  onSortChange();
});
</script>

<template>
  <div class="session-filters">
    <label>
      <span>{{ t("sessions.filterStatus", "Status") }}</span>
      <select v-model="status" class="select-input" @change="onStatusChange($event)">
        <option value="all">{{ t("sessions.filterAll", "All") }}</option>
        <option value="active">{{ t("sessions.statusActive", "Active") }}</option>
        <option value="ended">{{ t("sessions.statusEnded", "Ended") }}</option>
      </select>
    </label>
    <label>
      <span>{{ t("sessions.sortBy", "Sort by") }}</span>
      <select v-model="sort" class="select-input" @change="onSortChange($event)">
        <option value="last_activity">{{ t("sessions.sortLastActivity", "Last Activity") }}</option>
        <option value="started_at">{{ t("sessions.sortStartedAt", "Started At") }}</option>
      </select>
    </label>
  </div>
</template>

<style scoped>
.session-filters {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

label {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

label span {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.select-input {
  height: 36px;
  border-radius: 8px;
  border: 1px solid var(--color-border);
  background: var(--color-bg-input);
  color: var(--color-text-primary);
  font-family: inherit;
  padding: 0 10px;
  font-size: 13px;
}
</style>
