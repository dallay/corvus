<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useAdmin } from "@/composables/useAdmin";
import type { AdminCerebroSearchResult, AdminCerebroStatusResponse } from "@/types/admin-sessions";

const props = defineProps<{
  gatewayUrl: (path: string) => string;
  authHeaders: () => Record<string, string>;
  status: AdminCerebroStatusResponse | null;
}>();

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const emit = defineEmits<(e: "select", result: AdminCerebroSearchResult) => void>();

const admin = useAdmin(props.gatewayUrl, props.authHeaders);
const query = ref("");
// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const effectiveStatus = computed(() => props.status ?? admin.cerebroStatus.value);

onMounted(async () => {
  if (!props.status) {
    await admin.fetchCerebroStatus();
  }
});

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function submitSearch() {
  if (!query.value.trim()) {
    return;
  }
  await admin.searchCerebro({ query: query.value.trim(), limit: 8 });
}
</script>

<template>
  <section class="panel">
    <header class="panel-header">
      <div>
        <p class="eyebrow">Cerebro Memory</p>
        <h4>Semantic Search</h4>
      </div>
    </header>

    <div class="search-row">
      <input v-model="query" type="text" placeholder="Search long-term memory" />
      <button
        :disabled="effectiveStatus?.tools?.mem_search?.state !== 'available' || admin.loadingBuckets.value.cerebroSearch"
        @click="submitSearch"
      >
        Search
      </button>
    </div>

    <p v-if="effectiveStatus?.tools?.mem_search?.state !== 'available'" class="helper">
      {{ effectiveStatus?.tools?.mem_search?.message ?? "Semantic search is not currently available." }}
    </p>

    <p
      v-else-if="admin.loadingBuckets.value.cerebroSearch"
      class="helper"
      aria-live="polite"
      role="status"
    >
      Searching Cerebro…
    </p>

    <p v-else-if="admin.cerebroSearch.value && 'state' in admin.cerebroSearch.value && admin.cerebroSearch.value.state !== 'available'" class="helper">
      {{ admin.cerebroSearch.value.message }}
    </p>

    <ul
      v-else-if="admin.cerebroSearch.value && 'results' in admin.cerebroSearch.value"
      class="result-list"
    >
      <li v-for="result in admin.cerebroSearch.value.results" :key="result.memory_id" class="result-row">
        <button class="result-btn" @click="emit('select', result)">
          <strong>{{ result.summary }}</strong>
          <span class="meta">
            {{ result.topic_key ?? result.memory_id }}
            <template v-if="typeof result.score === 'number'"> · {{ result.score.toFixed(2) }}</template>
          </span>
        </button>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.panel {
  border: 1px solid var(--color-border);
  border-radius: 12px;
  padding: 14px;
}

.panel-header h4,
.eyebrow {
  margin: 0;
}

.eyebrow {
  font-size: 11px;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.search-row {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.search-row input {
  flex: 1;
  min-width: 0;
}

.result-list {
  list-style: none;
  padding: 0;
  margin: 12px 0 0;
  display: grid;
  gap: 8px;
}

.result-btn {
  width: 100%;
  text-align: left;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  padding: 10px;
  background: var(--color-bg-secondary);
}

.meta {
  display: block;
  margin-top: 4px;
  color: var(--color-text-secondary);
  font-size: 12px;
}
</style>
