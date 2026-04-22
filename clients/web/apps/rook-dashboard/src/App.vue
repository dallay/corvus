<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

import { RookApiClient, type RookApi } from "@/lib/api/client";
import { normalizeHashRoute, toHashRoute, type RookRoute } from "@/lib/navigation/routes";
import { useRookSession } from "@/lib/session/useRookSession";

import AccountsPage from "./features/accounts/AccountsPage.vue";
import OverviewPage from "./features/overview/OverviewPage.vue";

const { baseUrl, bearerToken, isConfigured } = useRookSession();
const route = ref<RookRoute>(normalizeHashRoute(window.location.hash));
const isConnected = ref(false);
const connectedClient = ref<RookApi | null>(null);

const client = computed<RookApi | null>(() => connectedClient.value);

function refreshConnectedClient() {
  if (!isConnected.value || !isConfigured.value) {
    connectedClient.value = null;
    return;
  }

  connectedClient.value = new RookApiClient(baseUrl.value.trim(), bearerToken.value.trim());
}

function connectSession() {
  isConnected.value = true;
  refreshConnectedClient();
}

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
          This slice is intentionally narrow: shell, overview, and provider/account administration.
          It does not expand the legacy Corvus dashboard and it does not absorb #593/#594.
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
      <div class="deferred-card">
        <strong>Deferred areas</strong>
        <span>Pools, routes, richer health ops (#593) and usage/logs/settings/backups (#594).</span>
      </div>
    </nav>

    <section v-if="!client" class="empty-state setup-state">
      <h2>Connect the dashboard to a Rook admin API</h2>
      <p>Enter the Rook base URL and bearer token to unlock overview and provider/account workflows.</p>
    </section>
    <OverviewPage v-else-if="route === 'overview'" :client="client" />
    <AccountsPage v-else :client="client" />
  </main>
</template>
