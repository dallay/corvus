import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type { AccountView, HealthAccountView, HealthSummaryView } from "@/lib/api/types";

import OverviewPage from "./OverviewPage.vue";

function createClient(overrides?: Partial<RookApiClient>): RookApiClient {
  return {
    listAccounts: vi.fn(async () => [] as AccountView[]),
    getHealthSummary: vi.fn(
      async () =>
        ({
          total: 0,
          healthy: 0,
          degraded: 0,
          unhealthy: 0,
          unknown: 0,
        }) as HealthSummaryView
    ),
    listAccountHealth: vi.fn(async () => [] as HealthAccountView[]),
    getAccount: vi.fn(),
    createAccount: vi.fn(),
    updateAccount: vi.fn(),
    deleteAccount: vi.fn(),
    ...overrides,
  } as unknown as RookApiClient;
}

describe("OverviewPage", () => {
  it("shows loading state while account-first overview requests are pending", async () => {
    let resolveAccounts: ((value: AccountView[]) => void) | undefined;
    const client = createClient({
      listAccounts: vi.fn(
        () =>
          new Promise<AccountView[]>((resolve) => {
            resolveAccounts = resolve;
          })
      ),
    });

    const wrapper = mount(OverviewPage, {
      props: { client },
      attachTo: document.body,
    });

    await Promise.resolve();
    expect(wrapper.text()).toContain("Loading overview…");

    resolveAccounts?.([]);
    await flushPromises();
  });

  it("renders recoverable error state and retries from existing read-only endpoints", async () => {
    const listAccounts = vi
      .fn()
      .mockRejectedValueOnce(new Error("overview failed"))
      .mockResolvedValueOnce([
        {
          id: "account-1",
          vendor: "open_ai",
          display_name: "Primary OpenAI",
          api_base_override: null,
          has_api_key: true,
          enabled: true,
          weight: 1,
          priority: 0,
          tags: [],
          capabilities: ["chat"],
        },
      ]);
    const client = createClient({ listAccounts });

    const wrapper = mount(OverviewPage, {
      props: { client },
      attachTo: document.body,
    });
    await flushPromises();

    expect(wrapper.text()).toContain("overview failed");

    await wrapper.get(".secondary-button").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("Total accounts");
    expect(listAccounts).toHaveBeenCalledTimes(2);
  });
});
