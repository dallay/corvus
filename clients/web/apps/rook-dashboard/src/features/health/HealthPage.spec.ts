import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type { AccountView, HealthAccountView, HealthSummaryView } from "@/lib/api/types";

import HealthPage from "./HealthPage.vue";

function createAccount(overrides: Partial<AccountView> = {}): AccountView {
  return {
    id: "account-1",
    vendor: "open_ai",
    display_name: "Primary OpenAI",
    api_base_override: null,
    has_api_key: true,
    enabled: true,
    weight: 1,
    priority: 0,
    tags: ["prod"],
    capabilities: ["chat"],
    ...overrides,
  };
}

function createHealth(overrides: Partial<HealthAccountView> = {}): HealthAccountView {
  return {
    account_id: "account-1",
    display_name: "Primary OpenAI",
    vendor: "open_ai",
    enabled: true,
    status: "healthy",
    last_checked: null,
    consecutive_failures: 0,
    cooldown_until: null,
    is_available: true,
    ...overrides,
  };
}

function createSummary(overrides: Partial<HealthSummaryView> = {}): HealthSummaryView {
  return {
    total: 1,
    healthy: 1,
    degraded: 0,
    unhealthy: 0,
    unknown: 0,
    ...overrides,
  };
}

function createClient(overrides?: Partial<RookApiClient>): RookApiClient {
  return {
    listAccounts: vi.fn(async () => [createAccount()]),
    getAccount: vi.fn(),
    listAccountHealth: vi.fn(async () => [createHealth()]),
    getHealthSummary: vi.fn(async () => createSummary()),
    createAccount: vi.fn(),
    updateAccount: vi.fn(),
    deleteAccount: vi.fn(),
    listPools: vi.fn(),
    getPool: vi.fn(),
    createPool: vi.fn(),
    updatePool: vi.fn(),
    deletePool: vi.fn(),
    addPoolMember: vi.fn(),
    removePoolMember: vi.fn(),
    listRoutes: vi.fn(),
    getRoute: vi.fn(),
    createRoute: vi.fn(),
    updateRoute: vi.fn(),
    deleteRoute: vi.fn(),
    ...overrides,
  } as unknown as RookApiClient;
}

async function mountPage(client: RookApiClient) {
  const wrapper = mount(HealthPage, {
    props: { client },
    attachTo: document.body,
  });
  await flushPromises();
  return wrapper;
}

describe("HealthPage", () => {
  it("shows summary and account visibility together", async () => {
    const wrapper = await mountPage(createClient());

    expect(wrapper.text()).toContain("Read-only health visibility");
    expect(wrapper.text()).toContain("Primary OpenAI");
    expect(wrapper.text()).toContain("healthy");
  });

  it("shows unknown status without implying historical data", async () => {
    const wrapper = await mountPage(
      createClient({ listAccountHealth: vi.fn(async () => [createHealth({ status: "unknown" })]) })
    );

    expect(wrapper.text()).toContain("unknown");
    expect(wrapper.text()).not.toContain("history");
  });

  it("shows empty state for total zero and no account rows", async () => {
    const wrapper = await mountPage(
      createClient({
        listAccounts: vi.fn(async () => []),
        listAccountHealth: vi.fn(async () => []),
        getHealthSummary: vi.fn(async () => createSummary({ total: 0, healthy: 0 })),
      })
    );

    expect(wrapper.text()).toContain("No current account health data");
    expect(wrapper.text()).toContain("read-only");
  });

  it("shows error state when health requests fail", async () => {
    const wrapper = await mountPage(
      createClient({
        listAccountHealth: vi.fn(async () => {
          throw new Error("health accounts unavailable");
        }),
      })
    );

    expect(wrapper.text()).toContain("health accounts unavailable");
  });

  it("omits unsupported remediation controls", async () => {
    const wrapper = await mountPage(createClient());

    expect(wrapper.text()).not.toContain("Retry health");
    expect(wrapper.text()).not.toContain("Acknowledge");
    expect(wrapper.text()).not.toContain("Reconnect");
  });
});
