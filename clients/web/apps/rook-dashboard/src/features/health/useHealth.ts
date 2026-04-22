import { computed, ref } from "vue";

import type { RookApi } from "@/lib/api/client";
import type { AccountView, HealthAccountView, HealthSummaryView } from "@/lib/api/types";

export interface HealthDisplayRow extends HealthAccountView {
  account: AccountView | null;
}

export function buildHealthRows(
  accounts: AccountView[],
  healthRows: HealthAccountView[]
): HealthDisplayRow[] {
  const accountsById = new Map(accounts.map((account) => [account.id, account]));

  return healthRows.map((row) => ({
    ...row,
    account: accountsById.get(row.account_id) ?? null,
  }));
}

export function useHealth(client: RookApi) {
  const accounts = ref<AccountView[]>([]);
  const accountHealth = ref<HealthAccountView[]>([]);
  const summary = ref<HealthSummaryView | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const rows = computed(() => buildHealthRows(accounts.value, accountHealth.value));
  const isEmpty = computed(() => (summary.value?.total ?? 0) === 0 && rows.value.length === 0);

  async function load() {
    loading.value = true;
    error.value = null;

    try {
      const [nextAccounts, nextHealthRows, nextSummary] = await Promise.all([
        client.listAccounts(),
        client.listAccountHealth(),
        client.getHealthSummary(),
      ]);
      accounts.value = nextAccounts;
      accountHealth.value = nextHealthRows;
      summary.value = nextSummary;
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      loading.value = false;
    }
  }

  return {
    accounts,
    accountHealth,
    error,
    isEmpty,
    load,
    loading,
    rows,
    summary,
  };
}
