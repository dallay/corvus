<script setup lang="ts">
import { computed, onMounted } from "vue";

import { useUsage } from "./useUsage";

const props = defineProps<{
  client: import("@/lib/api/client").RookApi;
}>();

const usageState = useUsage(props.client);

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const usage = computed(() => usageState.usage.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const loading = computed(() => usageState.loading.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const error = computed(() => usageState.error.value);

onMounted(async () => {
  await usageState.load();
});
</script>

<template>
  <section class="page-card">
    <header class="section-header">
      <div>
        <p class="eyebrow">Usage</p>
        <h2>Usage placeholder</h2>
        <p class="section-copy">
          This page reflects only the verified <code>GET /api/usage</code> placeholder contract.
          It does not invent totals, quotas, trends, costs, or provider breakdowns.
        </p>
      </div>
      <button class="secondary-button" type="button" @click="usageState.load">Refresh</button>
    </header>

    <p v-if="loading" class="state-banner info">Loading usage status…</p>
    <p v-else-if="error" class="state-banner danger">{{ error }}</p>
    <article v-else-if="usage" class="detail-card">
      <p class="eyebrow">Verified contract state</p>
      <h3>{{ usage.available ? "Usage status available" : "Usage data currently unavailable" }}</h3>
      <dl class="detail-grid">
        <div>
          <dt>Available</dt>
          <dd>{{ usage.available ? "Yes" : "No" }}</dd>
        </div>
        <div>
          <dt>Reason</dt>
          <dd>{{ usage.reason }}</dd>
        </div>
      </dl>
    </article>
  </section>
</template>
