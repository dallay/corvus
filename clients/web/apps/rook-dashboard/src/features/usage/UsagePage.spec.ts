import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type { UsageStatusView } from "@/lib/api/types";

import UsagePage from "./UsagePage.vue";

function createUsage(overrides: Partial<UsageStatusView> = {}): UsageStatusView {
  return {
    available: false,
    reason: "usage accounting is not implemented in M1",
    ...overrides,
  };
}

function createClient(overrides?: Partial<RookApiClient>): RookApiClient {
  return {
    listAccounts: vi.fn(),
    getAccount: vi.fn(),
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
    listAccountHealth: vi.fn(),
    getHealthSummary: vi.fn(),
    createAccount: vi.fn(),
    updateAccount: vi.fn(),
    deleteAccount: vi.fn(),
    getUsage: vi.fn(async () => createUsage()),
    getSettings: vi.fn(),
    updateSettings: vi.fn(),
    ...overrides,
  } as unknown as RookApiClient;
}

describe("UsagePage", () => {
  it("renders the placeholder usage response and avoids fake analytics copy", async () => {
    const wrapper = mount(UsagePage, {
      props: { client: createClient() },
      attachTo: document.body,
    });

    await flushPromises();

    expect(wrapper.text()).toContain("Usage placeholder");
    expect(wrapper.text()).toContain("usage accounting is not implemented in M1");
    expect(wrapper.text()).not.toContain("Total requests");
    expect(wrapper.text()).not.toContain("analytics");
  });

  it("shows loading state while usage is pending", async () => {
    let resolveUsage: ((value: UsageStatusView) => void) | undefined;
    const wrapper = mount(UsagePage, {
      props: {
        client: createClient({
          getUsage: vi.fn(
            () =>
              new Promise<UsageStatusView>((resolve) => {
                resolveUsage = resolve;
              })
          ),
        }),
      },
      attachTo: document.body,
    });

    await Promise.resolve();
    expect(wrapper.text()).toContain("Loading usage status…");

    resolveUsage?.(createUsage());
    await flushPromises();
  });

  it("shows recoverable error state and retries usage loading", async () => {
    const getUsage = vi
      .fn()
      .mockRejectedValueOnce(new Error("usage request failed"))
      .mockResolvedValueOnce(createUsage({ reason: "still placeholder" }));
    const wrapper = mount(UsagePage, {
      props: { client: createClient({ getUsage }) },
      attachTo: document.body,
    });

    await flushPromises();
    expect(wrapper.text()).toContain("usage request failed");

    await wrapper.get(".secondary-button").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("still placeholder");
    expect(getUsage).toHaveBeenCalledTimes(2);
  });
});
