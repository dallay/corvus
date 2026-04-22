import { computed, ref } from "vue";

import type { RookApi } from "@/lib/api/client";
import type { AccountView, HealthAccountView, HealthSummaryView } from "@/lib/api/types";

export interface ProviderGroupSummary {
  vendor: string;
  totalAccounts: number;
  enabledAccounts: number;
  disabledAccounts: number;
  healthyAccounts: number;
  degradedAccounts: number;
  unhealthyAccounts: number;
  unknownAccounts: number;
}

export function buildProviderGroupSummaries(
  accounts: AccountView[],
  healthRows: HealthAccountView[]
): ProviderGroupSummary[] {
  const healthByAccountId = new Map(healthRows.map((row) => [row.account_id, row]));
  const groups = new Map<string, ProviderGroupSummary>();

  for (const account of accounts) {
    const existing = groups.get(account.vendor) ?? {
      vendor: account.vendor,
      totalAccounts: 0,
      enabledAccounts: 0,
      disabledAccounts: 0,
      healthyAccounts: 0,
      degradedAccounts: 0,
      unhealthyAccounts: 0,
      unknownAccounts: 0,
    };

    existing.totalAccounts += 1;
    if (account.enabled) {
      existing.enabledAccounts += 1;
    } else {
      existing.disabledAccounts += 1;
    }

    const status = healthByAccountId.get(account.id)?.status ?? "unknown";
    if (status === "healthy") existing.healthyAccounts += 1;
    if (status === "degraded") existing.degradedAccounts += 1;
    if (status === "unhealthy") existing.unhealthyAccounts += 1;
    if (status === "unknown") existing.unknownAccounts += 1;

    groups.set(account.vendor, existing);
  }

  return [...groups.values()].sort((left, right) => left.vendor.localeCompare(right.vendor));
}

export function useOverview(client: RookApi) {
  const accounts = ref<AccountView[]>([]);
  const healthRows = ref<HealthAccountView[]>([]);
  const healthSummary = ref<HealthSummaryView | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const totalAccounts = computed(() => accounts.value.length);
  const enabledAccounts = computed(() => accounts.value.filter((account) => account.enabled).length);
  const disabledAccounts = computed(() => totalAccounts.value - enabledAccounts.value);
  const providerCount = computed(() => new Set(accounts.value.map((account) => account.vendor)).size);
  const providerGroups = computed(() => buildProviderGroupSummaries(accounts.value, healthRows.value));
  const isEmpty = computed(() => accounts.value.length === 0);

  async function load() {
    loading.value = true;
    error.value = null;

    try {
      const [nextAccounts, nextHealthSummary, nextHealthRows] = await Promise.all([
        client.listAccounts(),
        client.getHealthSummary(),
        client.listAccountHealth(),
      ]);
      accounts.value = nextAccounts;
      healthSummary.value = nextHealthSummary;
      healthRows.value = nextHealthRows;
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      loading.value = false;
    }
  }

  return {
    accounts,
    disabledAccounts,
    enabledAccounts,
    error,
    healthRows,
    healthSummary,
    isEmpty,
    load,
    loading,
    providerCount,
    providerGroups,
    totalAccounts,
  };
}
