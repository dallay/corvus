import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type { SettingsView } from "@/lib/api/types";

import { useSettings } from "./useSettings";

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

describe("useSettings", () => {
  it("loads defaults or persisted settings into current and draft state", async () => {
    const settings = useSettings(createClient());

    await settings.load();

    expect(settings.current.value).toEqual(createSettings());
    expect(settings.draft.value).toEqual(createSettings());
    expect(settings.isDirty.value).toBe(false);
  });

  it("tracks dirty state and applies the PUT response as canonical state", async () => {
    const updateSettings = vi.fn(async (payload: SettingsView) =>
      createSettings({
        ...payload,
        gateway_port: 15432,
      })
    );
    const settings = useSettings(createClient({ updateSettings }));

    await settings.load();
    settings.draft.value = createSettings({ gateway_port: 13000, log_json: true });

    expect(settings.isDirty.value).toBe(true);
    await settings.save();

    expect(updateSettings).toHaveBeenCalledWith(
      expect.objectContaining({ gateway_port: 13000, log_json: true })
    );
    expect(settings.current.value?.gateway_port).toBe(15432);
    expect(settings.draft.value?.gateway_port).toBe(15432);
    expect(settings.saveSuccess.value).toBe("Settings saved.");
    expect(settings.isDirty.value).toBe(false);
  });

  it("keeps draft state recoverable when the settings save fails", async () => {
    const settings = useSettings(
      createClient({
        updateSettings: vi.fn(async () => {
          throw new Error("gateway_port must be greater than 0");
        }),
      })
    );

    await settings.load();
    settings.draft.value = createSettings({ gateway_port: 0 });
    await settings.save();

    expect(settings.saveError.value).toBe("gateway_port must be greater than 0");
    expect(settings.draft.value?.gateway_port).toBe(0);
    expect(settings.current.value?.gateway_port).toBe(11434);
  });

  it("surfaces initial load failures and clears them on recovery", async () => {
    const getSettings = vi
      .fn()
      .mockRejectedValueOnce(new Error("settings unavailable"))
      .mockResolvedValueOnce(createSettings({ gateway_port: 12000 }));
    const settings = useSettings(createClient({ getSettings }));

    await settings.load();
    expect(settings.error.value).toBe("settings unavailable");

    await settings.load();
    expect(settings.error.value).toBeNull();
    expect(settings.current.value?.gateway_port).toBe(12000);
  });
});
