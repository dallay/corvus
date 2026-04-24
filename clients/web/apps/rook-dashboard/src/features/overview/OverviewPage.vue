<script setup lang="ts">
import { computed, onMounted } from "vue";

import type { RookApi } from "@/lib/api/client";

import { useOverview } from "./useOverview";

const props = defineProps<{
  client: RookApi;
}>();

const overview = useOverview(props.client);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const providerGroups = computed(() => overview.providerGroups.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const healthSummary = computed(() => overview.healthSummary.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const totalAccounts = computed(() => overview.totalAccounts.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const enabledAccounts = computed(() => overview.enabledAccounts.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const disabledAccounts = computed(() => overview.disabledAccounts.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const providerCount = computed(() => overview.providerCount.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const isEmpty = computed(() => overview.isEmpty.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const loading = computed(() => overview.loading.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const error = computed(() => overview.error.value);

onMounted(async () => {
  await overview.load();
});
</script>

<template>
  <section class="page-card">
    <header class="section-header">
      <div>
        <p class="eyebrow">Overview</p>
        <h2>Operator orientation</h2>
        <p class="section-copy">
          Built only from existing account and read-only health endpoints. Pools, routes, usage,
          logs, settings, and backups remain deferred to #593/#594.
        </p>
      </div>
      <button class="secondary-button" type="button" @click="overview.load">Retry</button>
    </header>

    <p v-if="loading" class="state-banner info">Loading overview…</p>
    <p v-else-if="error" class="state-banner danger">{{ error }}</p>
    <div v-else-if="isEmpty" class="empty-state">
      <h3>No configured provider accounts</h3>
      <p>Create your first provider account to start routing requests through Rook.</p>
    </div>
    <template v-else>
      <div class="summary-grid">
        <article class="summary-card">
          <span>Total accounts</span>
          <strong>{{ totalAccounts }}</strong>
        </article>
        <article class="summary-card">
          <span>Enabled</span>
          <strong>{{ enabledAccounts }}</strong>
        </article>
        <article class="summary-card">
          <span>Disabled</span>
          <strong>{{ disabledAccounts }}</strong>
        </article>
        <article class="summary-card">
          <span>Providers</span>
          <strong>{{ providerCount }}</strong>
        </article>
      </div>

      <div class="provider-overview-grid">
        <article v-for="group in providerGroups" :key="group.vendor" class="provider-card">
          <p class="provider-label">{{ group.vendor }}</p>
          <h3>{{ group.totalAccounts }} account<span v-if="group.totalAccounts !== 1">s</span></h3>
          <p class="provider-stats">
            {{ group.enabledAccounts }} enabled · {{ group.disabledAccounts }} disabled ·
            {{ group.healthyAccounts }} healthy · {{ group.degradedAccounts }} degraded ·
            {{ group.unhealthyAccounts }} unhealthy · {{ group.unknownAccounts }} unknown
          </p>
        </article>
      </div>

      <article v-if="healthSummary" class="detail-card">
        <p class="eyebrow">Read-only health summary</p>
        <h3>Current account health snapshot</h3>
        <dl class="detail-grid">
          <div>
            <dt>Healthy</dt>
            <dd>{{ healthSummary.healthy }}</dd>
          </div>
          <div>
            <dt>Degraded</dt>
            <dd>{{ healthSummary.degraded }}</dd>
          </div>
          <div>
            <dt>Unhealthy</dt>
            <dd>{{ healthSummary.unhealthy }}</dd>
          </div>
          <div>
            <dt>Unknown</dt>
            <dd>{{ healthSummary.unknown }}</dd>
          </div>
        </dl>
      </article>
    </template>
  </section>
</template>
