import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { computed, ref } from "vue";
import { createI18n } from "vue-i18n";

import { i18nConfig } from "@/i18n";
import type {
  AdminCostHistoryView,
  AdminCostSummaryView,
  AdminCostView,
} from "@/types/admin-config";

const hoisted = vi.hoisted(() => ({
  state: {} as Record<string, unknown>,
}));

vi.mock("@/composables/useCostGovernance", () => ({
  useCostGovernance: () => hoisted.state,
}));

vi.mock("@corvus/ui", () => ({
  Button: {
    template: "<button><slot /></button>",
  },
}));

import CostOverview from "@/components/config/CostOverview.vue";

function mountComponent() {
  return mount(CostOverview, {
    props: {
      gatewayUrl: "http://localhost:3000",
      bearerToken: "test-token",
    },
    global: {
      plugins: [createI18n({ ...i18nConfig, locale: "en" })],
    },
  });
}

function setBaseConfig(overrides: Partial<AdminCostView> = {}) {
  (hoisted.state.config as ReturnType<typeof ref<AdminCostView | null>>).value = {
    enabled: true,
    session_limit_usd: 25,
    daily_limit_usd: 50,
    monthly_limit_usd: 1000,
    warn_at_percent: 80,
    allow_override: true,
    ...overrides,
  };
}

describe("CostOverview", () => {
  beforeEach(() => {
    const config = ref<AdminCostView | null>(null);
    const summary = ref<AdminCostSummaryView | null>(null);
    const history = ref<AdminCostHistoryView | null>(null);
    const loading = ref(false);
    const error = ref<string | null>(null);
    const usageUnavailable = ref(false);
    const usageError = ref<string | null>(null);
    const actionMessage = ref<string | null>(null);
    const actionError = ref<string | null>(null);
    const actionPending = ref(false);
    const reload = vi.fn().mockResolvedValue(undefined);
    const grantOverride = vi.fn().mockImplementation(async () => {
      actionMessage.value = "Override granted: next_request";
    });
    const resetSession = vi.fn().mockImplementation(async () => {
      actionMessage.value = "Session totals reset: 6";
    });

    hoisted.state = {
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
      reload,
      grantOverride,
      resetSession,
      hasOperationalData: computed(() => summary.value !== null || history.value !== null),
      activeBudgetState: computed(() => summary.value?.budget_state ?? "allowed"),
    };

    setBaseConfig();
    (hoisted.state.summary as ReturnType<typeof ref<AdminCostSummaryView | null>>).value = null;
    (hoisted.state.history as ReturnType<typeof ref<AdminCostHistoryView | null>>).value = null;
    (hoisted.state.loading as ReturnType<typeof ref<boolean>>).value = false;
    (hoisted.state.error as ReturnType<typeof ref<string | null>>).value = null;
    (hoisted.state.usageUnavailable as ReturnType<typeof ref<boolean>>).value = false;
    (hoisted.state.usageError as ReturnType<typeof ref<string | null>>).value = null;
    (hoisted.state.actionMessage as ReturnType<typeof ref<string | null>>).value = null;
    (hoisted.state.actionError as ReturnType<typeof ref<string | null>>).value = null;
    (hoisted.state.actionPending as ReturnType<typeof ref<boolean>>).value = false;
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("falls back to config-only mode when live usage APIs are unavailable", async () => {
    (hoisted.state.usageUnavailable as ReturnType<typeof ref<boolean>>).value = true;
    (hoisted.state.usageError as ReturnType<typeof ref<string | null>>).value =
      "Live usage is unavailable. Showing saved policy only.";

    const wrapper = mountComponent();
    await flushPromises();

    expect(hoisted.state.reload).toHaveBeenCalled();
    expect(wrapper.find('[data-testid="cost-overview"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="cost-config-fallback"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("$50.00");
    expect(wrapper.text()).toContain("$1,000.00");
    expect(wrapper.text()).toContain("80%");
  });

  it("renders warning state with live summary and history", async () => {
    (hoisted.state.summary as ReturnType<typeof ref<AdminCostSummaryView | null>>).value = {
      session_cost_usd: 12.4,
      daily_cost_usd: 41,
      monthly_cost_usd: 320,
      total_tokens: 120044,
      request_count: 63,
      percent_used_session: 49.6,
      percent_used_daily: 82,
      percent_used_monthly: 32,
      budget_state: "warning",
      period: "day",
    };
    (hoisted.state.history as ReturnType<typeof ref<AdminCostHistoryView | null>>).value = {
      period: "day",
      points: [
        { bucket: "2026-04-04", cost_usd: 9.25, tokens: 18000, requests: 8 },
        { bucket: "2026-04-05", cost_usd: 11.7, tokens: 22000, requests: 10 },
      ],
      totals: { cost_usd: 20.95, tokens: 40000, requests: 18 },
    };

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find('[data-testid="cost-live-summary"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="cost-state-warning"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="cost-history"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("$25.00");
    expect(wrapper.text()).toContain("$12.40");
    expect(wrapper.text()).toContain("$41.00");
    expect(wrapper.text()).toContain("50%");
    expect(wrapper.text()).toContain("2026-04-04");
    expect(wrapper.text()).toContain("2026-04-05");
  });

  it("renders exceeded state when budget is blocked", async () => {
    (hoisted.state.summary as ReturnType<typeof ref<AdminCostSummaryView | null>>).value = {
      session_cost_usd: 30.5,
      daily_cost_usd: 55,
      monthly_cost_usd: 1005,
      total_tokens: 190000,
      request_count: 90,
      percent_used_daily: 110,
      percent_used_monthly: 100.5,
      budget_state: "exceeded",
      period: "month",
    };

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find('[data-testid="cost-state-exceeded"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("$1,005.00");
  });

  it("shows action affordances when operator actions are available", async () => {
    (hoisted.state.summary as ReturnType<typeof ref<AdminCostSummaryView | null>>).value = {
      session_cost_usd: 12.4,
      daily_cost_usd: 41,
      monthly_cost_usd: 320,
      total_tokens: 120044,
      request_count: 63,
      percent_used_daily: 82,
      percent_used_monthly: 32,
      budget_state: "warning",
      period: "day",
    };
    (hoisted.state.history as ReturnType<typeof ref<AdminCostHistoryView | null>>).value = {
      period: "day",
      points: [{ bucket: "2026-04-05", cost_usd: 11.7, tokens: 22000, requests: 10 }],
      totals: { cost_usd: 11.7, tokens: 22000, requests: 10 },
    };

    const wrapper = mountComponent();
    await flushPromises();

    await wrapper.get('[data-testid="cost-action-override"]').trigger("click");
    await wrapper.get('[data-testid="cost-action-reset-session"]').trigger("click");
    await flushPromises();

    expect(hoisted.state.grantOverride).toHaveBeenCalledTimes(1);
    expect(hoisted.state.resetSession).toHaveBeenCalledTimes(1);
  });

  it("keeps reset visible when overrides are disabled", async () => {
    setBaseConfig({ allow_override: false });
    (hoisted.state.summary as ReturnType<typeof ref<AdminCostSummaryView | null>>).value = {
      session_cost_usd: 4.2,
      daily_cost_usd: 10,
      monthly_cost_usd: 40,
      total_tokens: 1200,
      request_count: 4,
      percent_used_session: 16.8,
      percent_used_daily: 20,
      percent_used_monthly: 4,
      budget_state: "allowed",
      period: "session",
    };

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find('[data-testid="cost-actions"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="cost-action-override"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="cost-action-reset-session"]').exists()).toBe(true);
  });

  it("shows error on fetch failure", async () => {
    (hoisted.state.config as ReturnType<typeof ref<AdminCostView | null>>).value = null;
    (hoisted.state.error as ReturnType<typeof ref<string | null>>).value = "Network error";

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find(".error").exists()).toBe(true);
    expect(wrapper.text()).toContain("Network error");
  });

  it("shows error when cost data is missing from response", async () => {
    (hoisted.state.config as ReturnType<typeof ref<AdminCostView | null>>).value = null;
    (hoisted.state.error as ReturnType<typeof ref<string | null>>).value =
      "Cost data not available";

    const wrapper = mountComponent();
    await flushPromises();

    expect(wrapper.find(".error").exists()).toBe(true);
    expect(wrapper.text()).toContain("Cost data not available");
  });
});
