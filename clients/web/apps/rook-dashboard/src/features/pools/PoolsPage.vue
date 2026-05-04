<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";

import type { PoolView } from "@/lib/api/types";

import type { PoolFormInput } from "./usePools";
import { usePools } from "./usePools";

const props = defineProps<{
  client: import("@/lib/api/client").RookApi;
}>();

const {
  accountsById,
  actionError,
  addMember,
  create,
  detail,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  error,
  load,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  loading,
  membershipActionError,
  openDetail,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  poolOptions,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  pools,
  remove,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  removeMember,
  /* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
  saving,
  update,
} = usePools(props.client);

const mode = ref<"create" | "edit" | null>(null);
const pendingDeleteId = ref<string | null>(null);
const selectedMemberAccountId = ref("");
const validationError = ref<string | null>(null);
const editingPool = ref<PoolView | null>(null);
const form = reactive<PoolFormInput>({
  name: "",
  strategy: "round_robin",
  members: [],
  fallback_pool_id: null,
});

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
const availableAccounts = computed(() => [...accountsById.value.values()]);

onMounted(() => {
  void load();
});

function resetForm() {
  Object.assign(form, {
    name: "",
    strategy: "round_robin",
    members: [],
    fallback_pool_id: null,
  } satisfies PoolFormInput);
  selectedMemberAccountId.value = "";
  validationError.value = null;
}

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
function startCreate() {
  mode.value = "create";
  editingPool.value = null;
  resetForm();
}

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
async function startEdit(poolId: string) {
  mode.value = "edit";
  await openDetail(poolId);
  editingPool.value = detail.value;

  if (detail.value) {
    Object.assign(form, {
      name: detail.value.name,
      strategy: detail.value.strategy,
      members: [...detail.value.members],
      fallback_pool_id: detail.value.fallback_pool_id,
    } satisfies PoolFormInput);
  }
}

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
async function showDetail(poolId: string) {
  await openDetail(poolId);
}

function validateForm() {
  if (form.name.trim().length === 0) {
    validationError.value = "Pool name is required";
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

  if (mode.value === "create") {
    await create({ ...form, members: [...form.members] });
  }

  if (mode.value === "edit" && editingPool.value) {
    await update(editingPool.value.id, { ...form, members: [...form.members] });
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

/* biome-ignore lint/correctness/noUnusedVariables: used in Vue template */
async function submitAddMember() {
  if (!detail.value || selectedMemberAccountId.value.trim().length === 0) {
    return;
  }

  await addMember(detail.value.id, selectedMemberAccountId.value);
  if (!membershipActionError.value) {
    selectedMemberAccountId.value = "";
  }
}
</script>

<template>
  <section class="page-card">
    <header class="section-header">
      <div>
        <p class="eyebrow">Pools</p>
        <h2>Manage routing pools</h2>
        <p class="section-copy">
          Pool CRUD and membership flows stay inside the dedicated Rook dashboard surface using only
          existing pool and membership contracts.
        </p>
      </div>
      <button class="primary-button" data-testid="create-pool" type="button" @click="startCreate">
        Create pool
      </button>
    </header>

    <p v-if="loading" class="state-banner info">Loading pools…</p>
    <p v-else-if="error" class="state-banner danger">{{ error }}</p>
    <div v-else-if="pools.length === 0" class="empty-state">
      <h3>No pools configured yet</h3>
      <p>Create the first pool to start grouping provider accounts for route targets.</p>
    </div>
    <div v-else class="groups-grid">
      <article v-for="pool in pools" :key="pool.id" class="provider-card">
        <header class="provider-card__header">
          <div>
            <p class="provider-label">{{ pool.strategy }}</p>
            <h3>{{ pool.name }}</h3>
          </div>
        </header>
        <p class="provider-stats">{{ pool.members.length }} member<span v-if="pool.members.length !== 1">s</span></p>
        <div class="row-actions">
          <button data-testid="pool-detail-trigger" type="button" @click="showDetail(pool.id)">View detail</button>
          <button :data-testid="`edit-pool-${pool.id}`" type="button" @click="startEdit(pool.id)">Edit</button>
          <button data-testid="delete-pool" type="button" @click="pendingDeleteId = pool.id">Delete</button>
        </div>
      </article>
    </div>

    <aside v-if="detail" class="detail-card">
      <p class="eyebrow">Pool detail</p>
      <h3>{{ detail.name }}</h3>
      <dl class="detail-grid">
        <div>
          <dt>Pool id</dt>
          <dd>{{ detail.id }}</dd>
        </div>
        <div>
          <dt>Strategy</dt>
          <dd>{{ detail.strategy }}</dd>
        </div>
        <div>
          <dt>Fallback pool</dt>
          <dd>{{ detail.fallback_pool_id ?? "No fallback pool" }}</dd>
        </div>
      </dl>

      <section class="member-section">
        <h4>Members</h4>
        <ul class="account-list">
          <li v-for="memberId in detail.members" :key="memberId" class="account-row">
            <div>
              <strong>{{ accountsById.get(memberId)?.display_name ?? memberId }}</strong>
              <p class="account-meta">{{ memberId }}</p>
            </div>
            <div class="row-actions">
              <button
                :data-testid="`remove-member-${memberId}`"
                type="button"
                @click="removeMember(detail.id, memberId)"
              >
                Remove
              </button>
            </div>
          </li>
        </ul>

        <label>
          <span>Add existing account</span>
          <select v-model="selectedMemberAccountId" name="member-account-id">
            <option value="">Select account</option>
            <option v-for="account in availableAccounts" :key="account.id" :value="account.id">
              {{ account.display_name }}
            </option>
          </select>
        </label>
        <p v-if="membershipActionError" class="state-banner danger">{{ membershipActionError }}</p>
        <div class="form-actions">
          <button class="secondary-button" data-testid="add-member" type="button" @click="submitAddMember">
            Add member
          </button>
        </div>
      </section>
    </aside>

    <form v-if="mode" class="form-card" @submit.prevent="submit">
      <header>
        <p class="eyebrow">{{ mode === "create" ? "New pool" : "Edit pool" }}</p>
        <h3>{{ mode === "create" ? "Create pool" : "Edit pool" }}</h3>
      </header>

      <label>
        <span>Name</span>
        <input v-model="form.name" name="name" />
      </label>
      <label>
        <span>Strategy</span>
        <select v-model="form.strategy" name="strategy">
          <option value="round_robin">round_robin</option>
          <option value="priority">priority</option>
        </select>
      </label>
      <label>
        <span>Fallback pool</span>
        <select v-model="form.fallback_pool_id" name="fallback_pool_id">
          <option :value="null">No fallback pool</option>
          <option v-for="pool in poolOptions" :key="pool.id" :value="pool.id">{{ pool.name }}</option>
        </select>
      </label>
      <label>
        <span>Initial members</span>
        <select v-model="form.members" multiple name="initial-members">
          <option v-for="account in availableAccounts" :key="account.id" :value="account.id">
            {{ account.display_name }}
          </option>
        </select>
      </label>

      <p v-if="validationError" class="state-banner danger">{{ validationError }}</p>
      <p v-if="actionError" class="state-banner danger">{{ actionError }}</p>

      <div class="form-actions">
        <button class="primary-button" type="submit" :disabled="saving">Save pool</button>
        <button type="button" @click="mode = null">Cancel</button>
      </div>
    </form>

    <div v-if="pendingDeleteId" class="confirm-card" role="alertdialog" aria-modal="true">
      <h3>Delete pool?</h3>
      <p>Routes and fallback references must already be clear before this delete can succeed.</p>
      <p v-if="actionError" class="state-banner danger">{{ actionError }}</p>
      <div class="form-actions">
        <button class="danger-button" data-testid="confirm-delete-pool" type="button" @click="confirmDelete">Delete</button>
        <button type="button" @click="pendingDeleteId = null">Cancel</button>
      </div>
    </div>
  </section>
</template>
