import { computed, ref } from "vue";

import type { RookApi } from "@/lib/api/client";
import type { AccountView, CreatePoolRequest, PoolView, UpdatePoolRequest } from "@/lib/api/types";

export interface PoolFormInput {
  name: string;
  strategy: string;
  members: string[];
  fallback_pool_id: string | null;
}

export function dedupeMemberIds(memberIds: string[]): string[] {
  return [...new Set(memberIds.filter((memberId) => memberId.trim().length > 0))];
}

export function buildPoolUpdatePayload(input: PoolFormInput): UpdatePoolRequest {
  return {
    name: input.name.trim(),
    strategy: input.strategy,
    members: dedupeMemberIds(input.members),
    fallback_pool_id: input.fallback_pool_id,
  };
}

export function usePools(client: RookApi) {
  const pools = ref<PoolView[]>([]);
  const accounts = ref<AccountView[]>([]);
  const detail = ref<PoolView | null>(null);
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const actionError = ref<string | null>(null);
  const membershipActionError = ref<string | null>(null);

  const accountsById = computed(
    () => new Map(accounts.value.map((account) => [account.id, account]))
  );
  const poolOptions = computed(() => pools.value.map((pool) => ({ id: pool.id, name: pool.name })));

  async function load() {
    loading.value = true;
    error.value = null;

    try {
      const [nextPools, nextAccounts] = await Promise.all([
        client.listPools(),
        client.listAccounts(),
      ]);
      pools.value = nextPools.map((pool) => ({ ...pool, members: dedupeMemberIds(pool.members) }));
      accounts.value = nextAccounts;

      if (detail.value) {
        detail.value = pools.value.find((pool) => pool.id === detail.value?.id) ?? null;
      }
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      loading.value = false;
    }
  }

  async function openDetail(poolId: string) {
    detail.value = await client.getPool(poolId);
  }

  async function create(input: CreatePoolRequest) {
    saving.value = true;
    actionError.value = null;

    try {
      await client.createPool({
        ...input,
        members: dedupeMemberIds(input.members ?? []),
      });
      await load();
    } catch (createError) {
      actionError.value = createError instanceof Error ? createError.message : String(createError);
    } finally {
      saving.value = false;
    }
  }

  async function update(poolId: string, input: PoolFormInput) {
    saving.value = true;
    actionError.value = null;

    try {
      await client.updatePool(poolId, buildPoolUpdatePayload(input));
      await load();
      detail.value = await client.getPool(poolId);
    } catch (updateError) {
      actionError.value = updateError instanceof Error ? updateError.message : String(updateError);
    } finally {
      saving.value = false;
    }
  }

  async function remove(poolId: string) {
    saving.value = true;
    actionError.value = null;

    try {
      await client.deletePool(poolId);
      await load();
      if (detail.value?.id === poolId) {
        detail.value = null;
      }
    } catch (removeError) {
      actionError.value = removeError instanceof Error ? removeError.message : String(removeError);
    } finally {
      saving.value = false;
    }
  }

  async function addMember(poolId: string, accountId: string) {
    saving.value = true;
    membershipActionError.value = null;

    try {
      await client.addPoolMember(poolId, { account_id: accountId });
      await load();
      if (detail.value?.id === poolId) {
        detail.value = await client.getPool(poolId);
      }
    } catch (addError) {
      membershipActionError.value = addError instanceof Error ? addError.message : String(addError);
    } finally {
      saving.value = false;
    }
  }

  async function removeMember(poolId: string, accountId: string) {
    saving.value = true;
    membershipActionError.value = null;

    try {
      await client.removePoolMember(poolId, accountId);
      await load();
      if (detail.value?.id === poolId) {
        detail.value = await client.getPool(poolId);
      }
    } catch (removeError) {
      membershipActionError.value =
        removeError instanceof Error ? removeError.message : String(removeError);
    } finally {
      saving.value = false;
    }
  }

  return {
    accounts,
    accountsById,
    actionError,
    addMember,
    create,
    detail,
    error,
    load,
    loading,
    membershipActionError,
    openDetail,
    poolOptions,
    pools,
    remove,
    removeMember,
    saving,
    update,
  };
}
