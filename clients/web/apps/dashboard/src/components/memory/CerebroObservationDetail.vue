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

watch(
  () => props.selected?.memory_id,
  async (memoryId) => {
    if (memoryId) {
      await admin.fetchCerebroObservation(memoryId);
    }
  },
  { immediate: true }
);
</script>

<template>
  <section class="panel">
    <h4>Observation Detail</h4>
    <p v-if="!selected" class="helper">Pick a Cerebro result to inspect its full observation.</p>
    <p v-else-if="admin.loadingBuckets.value.cerebroObservation" class="helper">Loading detail…</p>
    <p
      v-else-if="admin.cerebroObservation.value && 'state' in admin.cerebroObservation.value && admin.cerebroObservation.value.state !== 'available'"
      class="helper"
    >
      {{ admin.cerebroObservation.value.message }}
    </p>
    <template
      v-else-if="admin.cerebroObservation.value && 'observation' in admin.cerebroObservation.value"
    >
      <pre class="payload">{{ JSON.stringify(admin.cerebroObservation.value.observation, null, 2) }}</pre>
      <div
        v-if="admin.cerebroObservation.value.observation.relationships || admin.cerebroObservation.value.observation.ontology"
        class="insights"
      >
        <h5>Relationship Insights</h5>
        <p class="helper">Read-only metadata returned by Cerebro.</p>
      </div>
    </template>
  </section>
</template>

<style scoped>
.panel {
  border: 1px solid var(--color-border);
  border-radius: 12px;
  padding: 14px;
}

.panel h4,
.insights h5 {
  margin: 0 0 8px;
}

.payload {
  margin: 0;
  padding: 10px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-bg-secondary) 85%, transparent);
  overflow: auto;
  font-size: 12px;
}

.insights {
  margin-top: 12px;
}
</style>
