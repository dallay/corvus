import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type { SettingsView } from "@/lib/api/types";

import SettingsPage from "./SettingsPage.vue";

function createSettings(overrides: Partial<SettingsView> = {}): SettingsView {
  return {
    gateway_port: 11434,
    default_routing_policy: {
      strategy: "round_robin",
      max_retries: 2,
      cooldown_seconds: 15,
    },
    log_json: false,
    log_level: "info",
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
    getUsage: vi.fn(),
    getSettings: vi.fn(async () => createSettings()),
    updateSettings: vi.fn(async (payload: SettingsView) => payload),
    ...overrides,
  } as unknown as RookApiClient;
}

describe("SettingsPage", () => {
  it("renders persisted or default settings values instead of an empty state", async () => {
    const wrapper = mount(SettingsPage, {
      props: { client: createClient() },
      attachTo: document.body,
    });

    await flushPromises();

    expect(wrapper.text()).toContain("Settings");
    expect((wrapper.get('input[name="gateway_port"]').element as HTMLInputElement).value).toBe(
      "11434"
    );
    expect(wrapper.text()).not.toContain("Import settings");
    expect(wrapper.text()).not.toContain("Export settings");
  });

  it("shows loading state while settings are pending", async () => {
    let resolveSettings: ((value: SettingsView) => void) | undefined;
    const wrapper = mount(SettingsPage, {
      props: {
        client: createClient({
          getSettings: vi.fn(
            () =>
              new Promise<SettingsView>((resolve) => {
                resolveSettings = resolve;
              })
          ),
        }),
      },
      attachTo: document.body,
    });

    await Promise.resolve();
    expect(wrapper.text()).toContain("Loading settings…");

    resolveSettings?.(createSettings());
    await flushPromises();
  });

  it("submits a full settings object through PUT and shows save progress", async () => {
    let resolveSave: ((value: SettingsView) => void) | undefined;
    const updateSettings = vi.fn(
      () =>
        new Promise<SettingsView>((resolve) => {
          resolveSave = resolve;
        })
    );
    const wrapper = mount(SettingsPage, {
      props: { client: createClient({ updateSettings }) },
      attachTo: document.body,
    });

    await flushPromises();
    await wrapper.get('input[name="gateway_port"]').setValue("12000");
    await wrapper.get('input[name="log_json"]').setValue();
    await wrapper.get("form").trigger("submit");
    await Promise.resolve();

    expect(wrapper.get('button[type="submit"]').text()).toContain("Saving…");
    expect(updateSettings).toHaveBeenCalledWith({
      gateway_port: 12000,
      default_routing_policy: {
        strategy: "round_robin",
        max_retries: 2,
        cooldown_seconds: 15,
      },
      log_json: true,
      log_level: "info",
    });

    resolveSave?.(createSettings({ gateway_port: 12000, log_json: true }));
    await flushPromises();

    expect(wrapper.text()).toContain("Settings saved.");
  });

  it("shows recoverable save errors without leaving settings context", async () => {
    const wrapper = mount(SettingsPage, {
      props: {
        client: createClient({
          updateSettings: vi.fn(async () => {
            throw new Error("gateway_port must be greater than 0");
          }),
        }),
      },
      attachTo: document.body,
    });

    await flushPromises();
    await wrapper.get('input[name="gateway_port"]').setValue("0");
    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(wrapper.text()).toContain("gateway_port must be greater than 0");
    expect(wrapper.text()).toContain("Save settings");
  });

  it("shows recoverable load errors and retries", async () => {
    const getSettings = vi
      .fn()
      .mockRejectedValueOnce(new Error("settings unavailable"))
      .mockResolvedValueOnce(createSettings({ gateway_port: 13000 }));
    const wrapper = mount(SettingsPage, {
      props: { client: createClient({ getSettings }) },
      attachTo: document.body,
    });

    await flushPromises();
    expect(wrapper.text()).toContain("settings unavailable");

    await wrapper.get(".secondary-button").trigger("click");
    await flushPromises();

    expect((wrapper.get('input[name="gateway_port"]').element as HTMLInputElement).value).toBe(
      "13000"
    );
  });
});
