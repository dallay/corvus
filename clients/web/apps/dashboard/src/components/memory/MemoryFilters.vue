<script setup lang="ts">
import { onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const props = defineProps<{
  initialSessionId?: string;
}>();

const emit = defineEmits<{
  (e: "update:category", value: string | undefined): void;
  (e: "update:sessionId", value: string | undefined): void;
  (e: "update:search", value: string | undefined): void;
}>();

const { t } = useI18n();
const category = ref<string>("all");
const sessionId = ref(props.initialSessionId ?? "");
const search = ref("");
let searchTimeout: ReturnType<typeof setTimeout> | undefined;

function onCategoryChange() {
  const value = category.value === "all" ? undefined : category.value;
  emit("update:category", value);
}

function onSessionIdChange() {
  const value = sessionId.value.trim() || undefined;
  emit("update:sessionId", value);
}

function onSearchInput() {
  clearTimeout(searchTimeout);
  searchTimeout = setTimeout(() => {
    const value = search.value.trim() || undefined;
    emit("update:search", value);
  }, 300);
}

onUnmounted(() => clearTimeout(searchTimeout));

watch(
  () => props.initialSessionId,
  (val) => {
    sessionId.value = val ?? "";
    onSessionIdChange();
  }
);
</script>

<template>
  <div class="memory-filters">
    <label>
      <span>{{ t("memory.filterCategory", "Category") }}</span>
      <select v-model="category" class="select-input" @change="onCategoryChange">
        <option value="all">{{ t("memory.filterAll", "All") }}</option>
        <option value="core">Core</option>
        <option value="daily">Daily</option>
        <option value="conversation">Conversation</option>
        <option value="custom">Custom</option>
      </select>
    </label>
    <label>
      <span>{{ t("memory.filterSessionId", "Session ID") }}</span>
      <input
        v-model="sessionId"
        type="text"
        class="text-input"
        :placeholder="t('memory.sessionIdPlaceholder', 'Filter by session…')"
        @change="onSessionIdChange"
      />
    </label>
    <label>
      <span>{{ t("memory.filterSearch", "Search") }}</span>
      <input
        v-model="search"
        type="text"
        class="text-input"
        :placeholder="t('memory.searchPlaceholder', 'Search content…')"
        @input="onSearchInput"
      />
    </label>
  </div>
</template>

<style scoped>
.memory-filters {
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

.select-input,
.text-input {
  height: 36px;
  border-radius: 8px;
  border: 1px solid var(--color-border);
  background: var(--color-bg-input);
  color: var(--color-text-primary);
  font-family: inherit;
  padding: 0 10px;
  font-size: 13px;
}

.text-input {
  min-width: 160px;
}
</style>
