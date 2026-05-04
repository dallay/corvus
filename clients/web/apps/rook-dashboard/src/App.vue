<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

import { type RookApi, RookApiClient } from "@/lib/api/client";
import { normalizeHashRoute, type RookRoute, toHashRoute } from "@/lib/navigation/routes";
import { useRookSession } from "@/lib/session/useRookSession";

/* biome-ignore lint/correctness/noUnusedImports: used in Vue template */
import AccountsPage from "./features/accounts/AccountsPage.vue";
/* biome-ignore lint/correctness/noUnusedImports: used in Vue template */
import HealthPage from "./features/health/HealthPage.vue";
/* biome-ignore lint/correctness/noUnusedImports: used in Vue template */
import OverviewPage from "./features/overview/OverviewPage.vue";
/* biome-ignore lint/correctness/noUnusedImports: used in Vue template */
import PoolsPage from "./features/pools/PoolsPage.vue";
/* biome-ignore lint/correctness/noUnusedImports: used in Vue template */
import RoutesPage from "./features/routes/RoutesPage.vue";
/* biome-ignore lint/correctness/noUnusedImports: used in Vue template */
import SettingsPage from "./features/settings/SettingsPage.vue";
/* biome-ignore lint/correctness/noUnusedImports: used in Vue template */
import UsagePage from "./features/usage/UsagePage.vue";

const { baseUrl, bearerToken, isConfigured } = useRookSession();
const route = ref<RookRoute>(normalizeHashRoute(window.location.hash));
const isConnected = ref(false);
const connectedClient = ref<RookApi | null>(null);

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const client = computed<RookApi | null>(() => connectedClient.value);

function refreshConnectedClient() {
  if (!isConnected.value || !isConfigured.value) {
    connectedClient.value = null;
    return;
  }

  connectedClient.value = new RookApiClient(baseUrl.value.trim(), bearerToken.value.trim());
}

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
function connectSession() {
  isConnected.value = true;
  refreshConnectedClient();
}

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
function disconnectSession() {
  isConnected.value = false;
  connectedClient.value = null;
}

watch([baseUrl, bearerToken], () => {
  if (isConnected.value) {
    refreshConnectedClient();
  }
});

function handleHashChange() {
  route.value = normalizeHashRoute(window.location.hash);
}

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
function navigate(nextRoute: RookRoute) {
  window.location.hash = toHashRoute(nextRoute);
}

onMounted(() => {
  if (!window.location.hash) {
    window.location.hash = toHashRoute("overview");
  }

  isConnected.value = isConfigured.value;
  refreshConnectedClient();

  window.addEventListener("hashchange", handleHashChange);
});

onBeforeUnmount(() => {
  window.removeEventListener("hashchange", handleHashChange);
});
</script>

<template>
  <main class="app-shell">
    <header class="hero-card">
      <div>
        <p class="eyebrow">Corvus Rook</p>
        <h1>Dedicated operator dashboard surface</h1>
        <p class="hero-copy">
          This slice extends the dedicated Rook dashboard surface with overview, provider/account,
          pool, route, read-only health, usage placeholder, and settings workflows. It does not
          expand the legacy Corvus dashboard and it does not invent unsupported #594 APIs.
        </p>
      </div>
      <section class="session-card">
        <label>
          <span>Rook base URL</span>
          <input v-model="baseUrl" placeholder="http://localhost:9090" type="url" />
        </label>
        <label>
          <span>Bearer token</span>
          <input v-model="bearerToken" placeholder="rook-admin-token" type="password" />
        </label>
        <p class="session-copy">
          Session values stay in <code>sessionStorage</code> only. The dashboard never persists or
          re-renders stored provider API keys.
        </p>
        <div class="form-actions">
          <button
            class="primary-button"
            data-testid="connect-session"
            type="button"
            @click="connectSession"
          >
            Connect
          </button>
          <button
            v-if="client"
            data-testid="disconnect-session"
            type="button"
            @click="disconnectSession"
          >
            Disconnect
          </button>
        </div>
      </section>
    </header>

    <nav class="nav-card" aria-label="Rook dashboard navigation">
      <button :class="['nav-button', { active: route === 'overview' }]" type="button" @click="navigate('overview')">
        Overview
      </button>
      <button :class="['nav-button', { active: route === 'accounts' }]" type="button" @click="navigate('accounts')">
        Providers &amp; accounts
      </button>
      <button :class="['nav-button', { active: route === 'pools' }]" type="button" @click="navigate('pools')">
        Pools
      </button>
      <button :class="['nav-button', { active: route === 'routes' }]" type="button" @click="navigate('routes')">
        Routes
      </button>
      <button :class="['nav-button', { active: route === 'health' }]" type="button" @click="navigate('health')">
        Health
      </button>
      <button :class="['nav-button', { active: route === 'usage' }]" type="button" @click="navigate('usage')">
        Usage
      </button>
      <button :class="['nav-button', { active: route === 'settings' }]" type="button" @click="navigate('settings')">
        Settings
      </button>
      <div class="deferred-card">
        <strong>Deferred areas</strong>
        <span>Logs and backups remain deferred until verified contracts exist.</span>
      </div>
    </nav>

    <section v-if="!client" class="empty-state setup-state">
      <h2>Connect the dashboard to a Rook admin API</h2>
      <p>
        Enter the Rook base URL and bearer token to unlock overview, providers/accounts, pools,
        routes, health, usage, and settings workflows.
      </p>
    </section>
    <OverviewPage v-else-if="route === 'overview'" :client="client" />
    <AccountsPage v-else-if="route === 'accounts'" :client="client" />
    <PoolsPage v-else-if="route === 'pools'" :client="client" />
    <RoutesPage v-else-if="route === 'routes'" :client="client" />
    <HealthPage v-else-if="route === 'health'" :client="client" />
    <UsagePage v-else-if="route === 'usage'" :client="client" />
    <SettingsPage v-else :client="client" />
  </main>
</template>
