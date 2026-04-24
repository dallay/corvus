<script setup lang="ts">
import { computed, onMounted } from "vue";

import { useSettings } from "./useSettings";

const props = defineProps<{
  client: import("@/lib/api/client").RookApi;
}>();

const settingsState = useSettings(props.client);

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const draft = computed(() => settingsState.draft.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const loading = computed(() => settingsState.loading.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const saving = computed(() => settingsState.saving.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const error = computed(() => settingsState.error.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const saveError = computed(() => settingsState.saveError.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const saveSuccess = computed(() => settingsState.saveSuccess.value);
/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const isDirty = computed(() => settingsState.isDirty.value);

onMounted(() => {
  void settingsState.load();
});
</script>

<template>
  <section class="page-card">
    <header class="section-header">
      <div>
        <p class="eyebrow">Settings</p>
        <h2>Manage Rook settings</h2>
        <p class="section-copy">
          This page uses only the verified <code>GET /api/settings</code> and
          <code>PUT /api/settings</code> singleton contract. It does not add PATCH, logs,
          backups, import, or export behavior.
        </p>
      </div>
      <button class="secondary-button" type="button" @click="settingsState.load">Refresh</button>
    </header>

    <p v-if="loading" class="state-banner info">Loading settings…</p>
    <p v-else-if="error" class="state-banner danger">{{ error }}</p>
    <form v-else-if="draft" class="form-card" @submit.prevent="settingsState.save">
      <label>
        <span>Gateway port</span>
        <input v-model.number="draft.gateway_port" min="1" name="gateway_port" type="number" />
      </label>
      <label>
        <span>Routing strategy</span>
        <select v-model="draft.default_routing_policy.strategy" name="strategy">
          <option value="round_robin">round_robin</option>
          <option value="priority">priority</option>
        </select>
      </label>
      <label>
        <span>Max retries</span>
        <input
          v-model.number="draft.default_routing_policy.max_retries"
          min="0"
          name="max_retries"
          type="number"
        />
      </label>
      <label>
        <span>Cooldown seconds</span>
        <input
          v-model.number="draft.default_routing_policy.cooldown_seconds"
          min="0"
          name="cooldown_seconds"
          type="number"
        />
      </label>
      <label class="checkbox-row">
        <input v-model="draft.log_json" name="log_json" type="checkbox" />
        <span>Enable JSON logs</span>
      </label>
      <label>
        <span>Log level</span>
        <input v-model.trim="draft.log_level" name="log_level" />
      </label>

      <p v-if="saveError" class="state-banner danger">{{ saveError }}</p>
      <p v-else-if="saveSuccess" class="state-banner success">{{ saveSuccess }}</p>

      <div class="form-actions">
        <button class="primary-button" type="submit" :disabled="saving || !isDirty">
          {{ saving ? "Saving…" : "Save settings" }}
        </button>
        <button type="button" :disabled="saving || !isDirty" @click="settingsState.resetDraft">
          Reset
        </button>
      </div>
    </form>
  </section>
</template>
