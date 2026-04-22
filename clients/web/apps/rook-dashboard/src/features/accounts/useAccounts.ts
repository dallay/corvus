import { computed, ref } from "vue";

import type { RookApi } from "@/lib/api/client";
import type {
  AccountView,
  CreateAccountRequest,
  HealthAccountView,
  UpdateAccountRequest,
} from "@/lib/api/types";

export interface AccountFormInput {
  vendor: string;
  display_name: string;
  api_base_override: string | null;
  api_key: string;
  enabled: boolean;
  weight: number;
  priority: number;
  tags: string[];
  capabilities: string[];
}

export interface AccountGroup {
  vendor: string;
  accounts: Array<AccountView & { health: HealthAccountView | null }>;
}

export function groupAccountsByVendor(
  accounts: AccountView[],
  healthRows: HealthAccountView[],
  selectedVendor: string | null = null
): AccountGroup[] {
  const healthByAccountId = new Map(healthRows.map((row) => [row.account_id, row]));
  const groups = new Map<string, AccountGroup>();

  for (const account of accounts) {
    if (selectedVendor && account.vendor !== selectedVendor) {
      continue;
    }

    const group = groups.get(account.vendor) ?? {
      vendor: account.vendor,
      accounts: [],
    };

    group.accounts.push({
      ...account,
      health: healthByAccountId.get(account.id) ?? null,
    });
    groups.set(account.vendor, group);
  }

  return [...groups.values()].sort((left, right) => left.vendor.localeCompare(right.vendor));
}

export function buildUpdatePayload(
  existing: AccountView,
  input: AccountFormInput
): UpdateAccountRequest {
  const payload: UpdateAccountRequest = {
    vendor: input.vendor,
    display_name: input.display_name,
    api_base_override: input.api_base_override,
    enabled: input.enabled,
    weight: input.weight,
    priority: input.priority,
    tags: input.tags,
    capabilities: input.capabilities,
  };

  const nextApiKey = input.api_key.trim();
  if (nextApiKey.length > 0 || !existing.has_api_key) {
    payload.api_key = nextApiKey;
  }

  return payload;
}

export function useAccounts(client: RookApi) {
  const accounts = ref<AccountView[]>([]);
  const healthRows = ref<HealthAccountView[]>([]);
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const actionError = ref<string | null>(null);
  const selectedVendor = ref<string | null>(null);

  const groups = computed(() =>
    groupAccountsByVendor(accounts.value, healthRows.value, selectedVendor.value)
  );

  async function load() {
    loading.value = true;
    error.value = null;

    try {
      const [nextAccounts, nextHealthRows] = await Promise.all([
        client.listAccounts(),
        client.listAccountHealth(),
      ]);
      accounts.value = nextAccounts;
      healthRows.value = nextHealthRows;
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      loading.value = false;
    }
  }

  async function create(payload: CreateAccountRequest) {
    saving.value = true;
    actionError.value = null;

    try {
      await client.createAccount(payload);
      await load();
    } catch (createError) {
      actionError.value = createError instanceof Error ? createError.message : String(createError);
    } finally {
      saving.value = false;
    }
  }

  async function update(existing: AccountView, input: AccountFormInput) {
    saving.value = true;
    actionError.value = null;

    try {
      await client.updateAccount(existing.id, buildUpdatePayload(existing, input));
      await load();
    } catch (updateError) {
      actionError.value = updateError instanceof Error ? updateError.message : String(updateError);
    } finally {
      saving.value = false;
    }
  }

  async function remove(accountId: string) {
    saving.value = true;
    actionError.value = null;

    try {
      await client.deleteAccount(accountId);
      await load();
    } catch (removeError) {
      actionError.value = removeError instanceof Error ? removeError.message : String(removeError);
    } finally {
      saving.value = false;
    }
  }

  return {
    accounts,
    actionError,
    create,
    error,
    groups,
    healthRows,
    load,
    loading,
    remove,
    saving,
    selectedVendor,
    update,
  };
}
