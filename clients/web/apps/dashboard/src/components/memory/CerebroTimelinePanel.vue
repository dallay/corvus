<script setup lang="ts">
import { watch } from "vue";
import { useAdmin } from "@/composables/useAdmin";
import type { AdminCerebroSearchResult } from "@/types/admin-sessions";

const props = defineProps<{
  gatewayUrl: (path: string) => string;
  authHeaders: () => Record<string, string>;
  selected: AdminCerebroSearchResult | null;
}>();

const admin = useAdmin(props.gatewayUrl, props.authHeaders);

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
function timelineItemKey(item: Record<string, unknown>): string {
  const stableValue = item.id ?? item.memory_id ?? item.timestamp ?? item.summary;
  return stableValue ? String(stableValue) : JSON.stringify(item);
}

watch(
  () => props.selected?.memory_id,
  async (memoryId) => {
    if (memoryId) {
      try {
        await admin.fetchCerebroTimeline({ memory_id: memoryId });
      } catch (error) {
        console.error("Failed to fetch Cerebro timeline", error);
      }
    }
  },
  { immediate: true }
);
</script>

<template>
  <section class="panel">
    <h4>Timeline</h4>
    <p v-if="!selected" class="helper">Timeline becomes available after you pick an observation.</p>
    <p v-else-if="admin.loadingBuckets.value.cerebroTimeline" class="helper">Loading timeline…</p>
    <p
      v-else-if="admin.cerebroTimeline.value && 'state' in admin.cerebroTimeline.value && admin.cerebroTimeline.value.state !== 'available'"
      class="helper"
    >
      {{ admin.cerebroTimeline.value.message }}
    </p>
    <ul v-else-if="admin.cerebroTimeline.value && 'items' in admin.cerebroTimeline.value" class="items">
      <li v-for="item in admin.cerebroTimeline.value.items" :key="timelineItemKey(item)">
        <pre>{{ JSON.stringify(item, null, 2) }}</pre>
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

.items {
  list-style: none;
  padding: 0;
  margin: 12px 0 0;
  display: grid;
  gap: 8px;
}

.items pre {
  margin: 0;
  padding: 10px;
  background: color-mix(in srgb, var(--color-bg-secondary) 85%, transparent);
  border-radius: 10px;
  overflow: auto;
  font-size: 12px;
}
</style>
