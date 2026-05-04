<script setup lang="ts">
import { computed, onMounted } from "vue";

import { useHealth } from "./useHealth";

const props = defineProps<{
  client: import("@/lib/api/client").RookApi;
}>();

const health = useHealth(props.client);

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const rows = computed(() => health.rows.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const summary = computed(() => health.summary.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const loading = computed(() => health.loading.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const error = computed(() => health.error.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const isEmpty = computed(() => health.isEmpty.value);

onMounted(async () => {
  await health.load();
});
</script>

<template>
  <section class="page-card">
    <header class="section-header">
      <div>
        <p class="eyebrow">Health</p>
        <h2>Read-only health visibility</h2>
        <p class="section-copy">
          This page is limited to verified account health and summary endpoints only. No remediation,
          reset, retry, reconnect, or #594 operational surfaces are introduced here.
        </p>
      </div>
      <button class="secondary-button" type="button" @click="health.load">Refresh</button>
    </header>

    <p v-if="loading" class="state-banner info">Loading health visibility…</p>
    <p v-else-if="error" class="state-banner danger">{{ error }}</p>
    <div v-else-if="isEmpty" class="empty-state">
      <h3>No current account health data</h3>
      <p>The verified health surface is read-only and currently has no account rows to display.</p>
    </div>
    <template v-else>
      <div v-if="summary" class="summary-grid">
        <article class="summary-card">
          <span>Total</span>
          <strong>{{ summary.total }}</strong>
        </article>
        <article class="summary-card">
          <span>Healthy</span>
          <strong>{{ summary.healthy }}</strong>
        </article>
        <article class="summary-card">
          <span>Degraded</span>
          <strong>{{ summary.degraded }}</strong>
        </article>
        <article class="summary-card">
          <span>Unhealthy</span>
          <strong>{{ summary.unhealthy }}</strong>
        </article>
        <article class="summary-card">
          <span>Unknown</span>
          <strong>{{ summary.unknown }}</strong>
        </article>
      </div>

      <article class="detail-card">
        <p class="eyebrow">Accounts</p>
        <h3>Current runtime snapshot</h3>
        <ul class="account-list">
          <li v-for="row in rows" :key="row.account_id" class="account-row">
            <div>
              <strong>{{ row.display_name }}</strong>
              <p class="account-meta">{{ row.vendor }} · {{ row.status }} · available: {{ row.is_available ? "yes" : "no" }}</p>
            </div>
            <div>
              <p class="account-meta">Last checked: {{ row.last_checked ?? "not yet checked" }}</p>
            </div>
          </li>
        </ul>
      </article>
    </template>
  </section>
</template>
