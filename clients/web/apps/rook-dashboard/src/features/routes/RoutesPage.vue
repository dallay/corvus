<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";

import type { RouteView } from "@/lib/api/types";

import type { RouteFormInput } from "./useRoutes";
import { useRoutes } from "./useRoutes";

const props = defineProps<{
  client: import("@/lib/api/client").RookApi;
}>();

const {
  actionError,
  create,
  detail,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  error,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  fallbackRouteOptions,
  load,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  loading,
  openDetail,
  pools,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  poolsById,
  remove,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  routes,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  saving,
  update,
} = useRoutes(props.client);

const mode = ref<"create" | "edit" | null>(null);
const editingRoute = ref<RouteView | null>(null);
const validationError = ref<string | null>(null);
const pendingDeleteId = ref<string | null>(null);
const form = reactive<RouteFormInput>({
  logical_model: "",
  target_pool_id: "",
  fallback_route_id: null,
  capability_constraints: [],
});

const poolOptions = computed(() => pools.value);

onMounted(() => {
  void load();
});

function resetForm() {
  Object.assign(form, {
    logical_model: "",
    target_pool_id: poolOptions.value[0]?.id ?? "",
    fallback_route_id: null,
    capability_constraints: [],
  } satisfies RouteFormInput);
  validationError.value = null;
}

function parseCapabilityConstraints(value: string): string[] {
  return value
    .split(",")
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0);
}

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const capabilityConstraintsText = computed({
  get: () => form.capability_constraints.join(", "),
  set: (value: string) => {
    form.capability_constraints = parseCapabilityConstraints(value);
  },
});

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
function startCreate() {
  mode.value = "create";
  editingRoute.value = null;
  resetForm();
}

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
async function startEdit(routeId: string) {
  mode.value = "edit";
  await openDetail(routeId);
  editingRoute.value = detail.value;

  if (detail.value) {
    Object.assign(form, {
      logical_model: detail.value.logical_model,
      target_pool_id: detail.value.target_pool_id,
      fallback_route_id: detail.value.fallback_route_id,
      capability_constraints: [...detail.value.capability_constraints],
    } satisfies RouteFormInput);
  }
}

function validateForm() {
  if (form.logical_model.trim().length === 0) {
    validationError.value = "Logical model is required";
    return false;
  }

  if (form.target_pool_id.trim().length === 0) {
    validationError.value = "Target pool is required";
    return false;
  }

  validationError.value = null;
  return true;
}

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
async function submit() {
  if (!validateForm()) {
    return;
  }

  const payload = {
    logical_model: form.logical_model.trim(),
    target_pool_id: form.target_pool_id,
    fallback_route_id: form.fallback_route_id,
    capability_constraints: [...form.capability_constraints],
  };

  if (mode.value === "create") {
    await create(payload);
  }

  if (mode.value === "edit" && editingRoute.value) {
    await update(editingRoute.value.id, payload);
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
    pendingDeleteId.value = null;
  }
}
</script>

<template>
  <section class="page-card">
    <header class="section-header">
      <div>
        <p class="eyebrow">Routes</p>
        <h2>Manage model routes</h2>
        <p class="section-copy">
          Route CRUD stays bound to verified route and pool ids from the dedicated Rook dashboard surface.
        </p>
      </div>
      <button class="primary-button" data-testid="create-route" type="button" @click="startCreate">
        Create route
      </button>
    </header>

    <p v-if="loading" class="state-banner info">Loading routes…</p>
    <p v-else-if="error" class="state-banner danger">{{ error }}</p>
    <div v-else-if="routes.length === 0" class="empty-state">
      <h3>No routes configured yet</h3>
      <p>Create the first route to connect logical models to existing pool ids.</p>
    </div>
    <div v-else class="groups-grid">
      <article v-for="route in routes" :key="route.id" class="provider-card">
        <header class="provider-card__header">
          <div>
            <p class="provider-label">{{ route.id }}</p>
            <h3>{{ route.logical_model }}</h3>
          </div>
        </header>
        <p class="provider-stats">
          Target pool: {{ poolsById.get(route.target_pool_id)?.name ?? route.target_pool_id }}
        </p>
        <div class="row-actions">
          <button :data-testid="`view-route-${route.id}`" type="button" @click="openDetail(route.id)">View detail</button>
          <button :data-testid="`edit-route-${route.id}`" type="button" @click="startEdit(route.id)">Edit</button>
          <button type="button" @click="pendingDeleteId = route.id">Delete</button>
        </div>
      </article>
    </div>

    <aside v-if="detail" class="detail-card">
      <p class="eyebrow">Route detail</p>
      <h3>{{ detail.logical_model }}</h3>
      <dl class="detail-grid">
        <div>
          <dt>Route id</dt>
          <dd>{{ detail.id }}</dd>
        </div>
        <div>
          <dt>Target pool</dt>
          <dd>{{ poolsById.get(detail.target_pool_id)?.name ?? detail.target_pool_id }}</dd>
        </div>
        <div>
          <dt>Fallback route</dt>
          <dd>{{ detail.fallback_route_id ?? "No fallback route" }}</dd>
        </div>
        <div>
          <dt>Capability constraints</dt>
          <dd>{{ detail.capability_constraints.length > 0 ? detail.capability_constraints.join(", ") : "No capability constraints" }}</dd>
        </div>
      </dl>
    </aside>

    <form v-if="mode" class="form-card" @submit.prevent="submit">
      <header>
        <p class="eyebrow">{{ mode === "create" ? "New route" : "Edit route" }}</p>
        <h3>{{ mode === "create" ? "Create route" : "Edit route" }}</h3>
      </header>

      <label>
        <span>Logical model</span>
        <input v-model="form.logical_model" name="logical_model" />
      </label>
      <label>
        <span>Target pool</span>
        <select v-model="form.target_pool_id" name="target_pool_id">
          <option v-for="pool in poolOptions" :key="pool.id" :value="pool.id">{{ pool.name }}</option>
        </select>
      </label>
      <label>
        <span>Fallback route</span>
        <select v-model="form.fallback_route_id" name="fallback_route_id">
          <option :value="null">No fallback route</option>
          <option v-for="route in fallbackRouteOptions" :key="route.id" :value="route.id">{{ route.logical_model }}</option>
        </select>
      </label>
      <label>
        <span>Capability constraints</span>
        <textarea v-model="capabilityConstraintsText" name="capability_constraints" rows="3"></textarea>
      </label>

      <p v-if="validationError" class="state-banner danger">{{ validationError }}</p>
      <p v-if="actionError" class="state-banner danger">{{ actionError }}</p>

      <div class="form-actions">
        <button class="primary-button" type="submit" :disabled="saving">Save route</button>
        <button type="button" @click="mode = null">Cancel</button>
      </div>
    </form>

    <div v-if="pendingDeleteId" class="confirm-card" role="alertdialog" aria-modal="true">
      <h3>Delete route?</h3>
      <p>Fallback references must already be clear before this delete can succeed.</p>
      <p v-if="actionError" class="state-banner danger">{{ actionError }}</p>
      <div class="form-actions">
        <button class="danger-button" type="button" @click="confirmDelete">Delete</button>
        <button type="button" @click="pendingDeleteId = null">Cancel</button>
      </div>
    </div>
  </section>
</template>
