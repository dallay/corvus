<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";

import type { AccountView, CreateAccountRequest } from "@/lib/api/types";

import type { AccountFormInput } from "./useAccounts";
import { useAccounts } from "./useAccounts";

const props = defineProps<{
  client: import("@/lib/api/client").RookApi;
}>();

const {
  actionError,
  create,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  error,
  groups,
  load,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  loading,
  remove,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  saving,
  update,
} = useAccounts(props.client);
const mode = ref<"create" | "edit" | null>(null);
const detail = ref<AccountView | null>(null);
const pendingDeleteId = ref<string | null>(null);
const form = reactive<AccountFormInput>({
  vendor: "open_ai",
  display_name: "",
  api_base_override: null,
  api_key: "",
  enabled: true,
  weight: 1,
  priority: 0,
  tags: [],
  capabilities: ["chat"],
});

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const visibleGroups = computed(() => groups.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const formTitle = computed(() =>
  mode.value === "create" ? "Create provider account" : "Edit provider account"
);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const credentialHint = computed(() => {
  if (mode.value !== "edit" || !detail.value) {
    return "API keys are write-only. The value you enter is sent once and never shown again.";
  }

  return detail.value.has_api_key
    ? "Stored API key exists. Leave the replacement field blank to preserve it."
    : "No API key is currently stored. Add one if this provider requires credentials.";
});

async function ensureLoaded() {
  await load();
}

onMounted(() => {
  void ensureLoaded();
});

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
function startCreate() {
  mode.value = "create";
  detail.value = null;
  Object.assign(form, {
    vendor: "open_ai",
    display_name: "",
    api_base_override: null,
    api_key: "",
    enabled: true,
    weight: 1,
    priority: 0,
    tags: [],
    capabilities: ["chat"],
  } satisfies AccountFormInput);
}

async function openDetail(accountId: string) {
  detail.value = await props.client.getAccount(accountId);
  if (mode.value === "edit") {
    applyDetailToForm(detail.value);
  }
}

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
async function startEdit(accountId: string) {
  mode.value = "edit";
  await openDetail(accountId);
}

function applyDetailToForm(account: AccountView) {
  Object.assign(form, {
    vendor: account.vendor,
    display_name: account.display_name,
    api_base_override: account.api_base_override,
    api_key: "",
    enabled: account.enabled,
    weight: account.weight,
    priority: account.priority,
    tags: [...account.tags],
    capabilities: [...account.capabilities],
  } satisfies AccountFormInput);
}

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
async function submit() {
  if (mode.value === "create") {
    await create({
      vendor: form.vendor,
      display_name: form.display_name,
      api_base_override: form.api_base_override,
      api_key: form.api_key || null,
      enabled: form.enabled,
      weight: form.weight,
      priority: form.priority,
      tags: [...form.tags],
      capabilities: [...form.capabilities],
    } satisfies CreateAccountRequest);
  }

  if (mode.value === "edit" && detail.value) {
    await update(detail.value, {
      ...form,
      tags: [...form.tags],
      capabilities: [...form.capabilities],
    });
    detail.value = await props.client.getAccount(detail.value.id);
  }

  if (!actionError.value) {
    mode.value = null;
  }
}

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
async function confirmDelete() {
  if (!pendingDeleteId.value) {
    return;
  }

  await remove(pendingDeleteId.value);
  if (!actionError.value) {
    if (detail.value?.id === pendingDeleteId.value) {
      detail.value = null;
    }
    pendingDeleteId.value = null;
  }
}
</script>

<template>
  <section class="page-card">
    <header class="section-header">
      <div>
        <p class="eyebrow">Providers &amp; accounts</p>
        <h2>Manage provider accounts</h2>
        <p class="section-copy">
          Grouped by vendor with redacted credential UX. Pools, routes, diagnostics, usage, logs,
          settings, and backups stay out of scope for #592.
        </p>
      </div>
      <button class="primary-button" type="button" :disabled="loading || saving" @click="startCreate">
        Create account
      </button>
    </header>

    <p v-if="loading" class="state-banner info">Loading provider accounts…</p>
    <p v-else-if="error" class="state-banner danger">{{ error }}</p>
    <div v-else-if="visibleGroups.length === 0" class="empty-state">
      <h3>No provider accounts match this view</h3>
      <p>Create your first account or change the selected provider filter.</p>
    </div>
    <div v-else class="groups-grid">
      <article v-for="group in visibleGroups" :key="group.vendor" class="provider-card">
        <header class="provider-card__header">
          <div>
            <p class="provider-label">{{ group.vendor }}</p>
            <h3>{{ group.accounts.length }} account<span v-if="group.accounts.length !== 1">s</span></h3>
          </div>
        </header>
        <ul class="account-list">
          <li v-for="account in group.accounts" :key="account.id" class="account-row">
            <div>
              <button class="link-button" type="button" @click="openDetail(account.id)">
                {{ account.display_name }}
              </button>
              <p class="account-meta">
                {{ account.enabled ? "Enabled" : "Disabled" }} ·
                {{ account.health?.status ?? "unknown" }} ·
                {{ account.has_api_key ? "stored key" : "no key" }}
              </p>
            </div>
            <div class="row-actions">
              <button type="button" @click="startEdit(account.id)">Edit</button>
              <button type="button" @click="pendingDeleteId = account.id">Delete</button>
            </div>
          </li>
        </ul>
      </article>
    </div>

    <aside v-if="detail" class="detail-card" aria-live="polite">
      <p class="eyebrow">Account detail</p>
      <h3>{{ detail.display_name }}</h3>
      <dl class="detail-grid">
        <div>
          <dt>Vendor</dt>
          <dd>{{ detail.vendor }}</dd>
        </div>
        <div>
          <dt>Enabled</dt>
          <dd>{{ detail.enabled ? "Enabled" : "Disabled" }}</dd>
        </div>
        <div>
          <dt>Credential status</dt>
          <dd>{{ detail.has_api_key ? "Stored API key exists" : "No API key stored" }}</dd>
        </div>
        <div>
          <dt>API base override</dt>
          <dd>{{ detail.api_base_override ?? "Default provider endpoint" }}</dd>
        </div>
      </dl>
    </aside>

    <form v-if="mode" class="form-card" @submit.prevent="submit">
      <header>
        <p class="eyebrow">{{ mode === "create" ? "New account" : "Edit account" }}</p>
        <h3>{{ formTitle }}</h3>
        <p class="section-copy">{{ credentialHint }}</p>
      </header>

      <label>
        <span>Vendor</span>
        <input v-model="form.vendor" name="vendor" />
      </label>
      <label>
        <span>Display name</span>
        <input v-model="form.display_name" name="display_name" />
      </label>
      <label>
        <span>API key</span>
        <input v-model="form.api_key" name="api_key" type="password" autocomplete="off" />
      </label>
      <label class="checkbox-row">
        <input v-model="form.enabled" name="enabled" type="checkbox" />
        <span>Account enabled</span>
      </label>

      <p v-if="actionError" class="state-banner danger">{{ actionError }}</p>

      <div class="form-actions">
        <button class="primary-button" type="submit" :disabled="saving">
          Save account
        </button>
        <button type="button" @click="mode = null">Cancel</button>
      </div>
    </form>

    <div v-if="pendingDeleteId" class="confirm-card" role="alertdialog" aria-modal="true">
      <h3>Delete provider account?</h3>
      <p>The account will be removed from this Rook surface. Existing provider tests are still deferred.</p>
      <p v-if="actionError" class="state-banner danger">{{ actionError }}</p>
      <div class="form-actions">
        <button class="danger-button" type="button" @click="confirmDelete">Delete</button>
        <button type="button" @click="pendingDeleteId = null">Cancel</button>
      </div>
    </div>
  </section>
</template>
