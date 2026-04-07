import { trimTrailingSlashes, validateGatewayUrl } from "@corvus/shared";
import { computed, ref } from "vue";
import { useAdmin } from "@/composables/useAdmin";
import type {
  AdminBudgetState,
  AdminCostHistoryView,
  AdminCostOverrideRecordView,
  AdminCostResetResultView,
  AdminCostSummaryView,
  AdminCostView,
  AdminDeprecatedFieldView,
} from "@/types/admin-config";

type Translator = (key: string) => string;

export function useCostGovernance(
  gatewayUrl: () => string,
  bearerToken: () => string,
  t: Translator
) {
  const config = ref<AdminCostView | null>(null);
  const summary = ref<AdminCostSummaryView | null>(null);
  const history = ref<AdminCostHistoryView | null>(null);
  const loading = ref(true);
  const error = ref<string | null>(null);
  const usageUnavailable = ref(false);
  const usageError = ref<string | null>(null);
  const actionMessage = ref<string | null>(null);
  const actionError = ref<string | null>(null);
  const actionPending = ref(false);
  const deprecations = ref<AdminDeprecatedFieldView[]>([]);

  const admin = useAdmin(buildGatewayUrl, authHeaders);

  function buildGatewayUrl(path: string): string {
    const base = validateGatewayUrl(gatewayUrl());
    if (!base) {
      throw new Error(t("errors.invalidGatewayUrl"));
    }

    const baseStr = trimTrailingSlashes(base.toString());
    return new URL(path.replace(/^\//, ""), `${baseStr}/`).toString();
  }

  function authHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };

    const token = bearerToken().trim();
    if (token) {
      headers.Authorization = `Bearer ${token}`;
    }

    return headers;
  }

  function updateDeprecations(): void {
    // Pending backend support: populate deprecations from admin config payloads
    // once fetchCostConfig() exposes cost-specific deprecation metadata.
    deprecations.value = [];
  }

  async function reload(): Promise<void> {
    loading.value = true;
    error.value = null;
    actionMessage.value = null;
    actionError.value = null;
    usageError.value = null;
    usageUnavailable.value = false;

    try {
      const nextConfig = await admin.fetchCostConfig();
      if (!nextConfig) {
        config.value = null;
        error.value = t("errors.costNotAvailable");
        return;
      }

      config.value = nextConfig;
      updateDeprecations();

      const [summaryResult, historyResult] = await Promise.allSettled([
        admin.fetchCostSummary(),
        admin.fetchCostHistory({ period: "day", window: 7 }),
      ]);

      if (summaryResult.status === "fulfilled") {
        summary.value = summaryResult.value.summary;
        config.value = summaryResult.value.config;
      } else {
        summary.value = null;
      }

      if (historyResult.status === "fulfilled") {
        history.value = historyResult.value;
      } else {
        history.value = null;
      }

      if (summaryResult.status === "rejected" || historyResult.status === "rejected") {
        usageUnavailable.value = true;
        usageError.value = t("cost.usageUnavailable");
      }
    } catch (e: unknown) {
      config.value = null;
      summary.value = null;
      history.value = null;
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      loading.value = false;
    }
  }

  async function grantOverride(): Promise<void> {
    actionPending.value = true;
    actionMessage.value = null;
    actionError.value = null;

    try {
      const result: AdminCostOverrideRecordView = await admin.grantCostOverride();
      actionMessage.value = `${t("cost.overrideGranted")}: ${result.scope}`;
      await reload();
    } catch (e: unknown) {
      actionError.value = e instanceof Error ? e.message : String(e);
    } finally {
      actionPending.value = false;
    }
  }

  async function resetSession(): Promise<void> {
    actionPending.value = true;
    actionMessage.value = null;
    actionError.value = null;

    try {
      const result: AdminCostResetResultView = await admin.resetCost("session");
      actionMessage.value = `${t("cost.sessionReset")}: ${result.removed_requests}`;
      await reload();
    } catch (e: unknown) {
      actionError.value = e instanceof Error ? e.message : String(e);
    } finally {
      actionPending.value = false;
    }
  }

  const hasOperationalData = computed(() => summary.value !== null || history.value !== null);

  const activeBudgetState = computed<AdminBudgetState>(
    () => summary.value?.budget_state ?? "allowed"
  );

  return {
    config,
    summary,
    history,
    loading,
    error,
    usageUnavailable,
    usageError,
    actionMessage,
    actionError,
    actionPending,
    deprecations,
    hasOperationalData,
    activeBudgetState,
    reload,
    grantOverride,
    resetSession,
  };
}
