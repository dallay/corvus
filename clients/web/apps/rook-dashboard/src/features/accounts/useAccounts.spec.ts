import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type {
  AccountView,
  CreateAccountRequest,
  HealthAccountView,
  UpdateAccountRequest,
} from "@/lib/api/types";

import { buildUpdatePayload, groupAccountsByVendor, useAccounts } from "./useAccounts";

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

function createClient(overrides?: Partial<RookApiClient>): RookApiClient {
  return {
    listAccounts: vi.fn(async () => [] as AccountView[]),
    getAccount: vi.fn(async () => createAccount()),
    listAccountHealth: vi.fn(async () => [] as HealthAccountView[]),
    getHealthSummary: vi.fn(),
    createAccount: vi.fn(async (payload: CreateAccountRequest) =>
      createAccount({
        id: "created-account",
        vendor: payload.vendor,
        display_name: payload.display_name,
        api_base_override: payload.api_base_override ?? null,
        has_api_key: Boolean(payload.api_key),
        enabled: payload.enabled ?? true,
        weight: payload.weight ?? 1,
        priority: payload.priority ?? 0,
        tags: payload.tags ?? [],
        capabilities: payload.capabilities ?? [],
      })
    ),
    updateAccount: vi.fn(async (accountId: string, payload: UpdateAccountRequest) =>
      createAccount({
        id: accountId,
        vendor: payload.vendor,
        display_name: payload.display_name,
        api_base_override: payload.api_base_override ?? null,
        has_api_key: true,
        enabled: payload.enabled,
        weight: payload.weight,
        priority: payload.priority,
        tags: payload.tags,
        capabilities: payload.capabilities,
      })
    ),
    deleteAccount: vi.fn(async () => undefined),
    ...overrides,
  } as unknown as RookApiClient;
}

describe("groupAccountsByVendor", () => {
  it("groups and filters accounts by vendor", () => {
    const groups = groupAccountsByVendor(
      [
        createAccount({ id: "a-1", vendor: "open_ai", display_name: "Primary" }),
        createAccount({ id: "a-2", vendor: "anthropic", display_name: "Claude", enabled: false }),
      ],
      [
        createHealth({ account_id: "a-1", vendor: "open_ai", status: "healthy" }),
        createHealth({ account_id: "a-2", vendor: "anthropic", status: "degraded" }),
      ],
      "anthropic"
    );

    expect(groups).toHaveLength(1);
    expect(groups[0]).toEqual(
      expect.objectContaining({
        vendor: "anthropic",
        accounts: [expect.objectContaining({ id: "a-2" })],
      })
    );
  });
});

describe("buildUpdatePayload", () => {
  it("omits unchanged api_key during edit payload creation", () => {
    expect(
      buildUpdatePayload(createAccount(), {
        vendor: "open_ai",
        display_name: "Primary OpenAI Updated",
        api_base_override: null,
        api_key: "",
        enabled: false,
        weight: 2,
        priority: 1,
        tags: ["prod", "edited"],
        capabilities: ["chat"],
      })
    ).toEqual({
      vendor: "open_ai",
      display_name: "Primary OpenAI Updated",
      api_base_override: null,
      enabled: false,
      weight: 2,
      priority: 1,
      tags: ["prod", "edited"],
      capabilities: ["chat"],
    });
  });
});

describe("useAccounts", () => {
  it("loads grouped accounts and health rows", async () => {
    const client = createClient({
      listAccounts: vi.fn(async () => [
        createAccount({ id: "a-1", vendor: "open_ai" }),
        createAccount({ id: "a-2", vendor: "anthropic", enabled: false }),
      ]),
      listAccountHealth: vi.fn(async () => [
        createHealth({ account_id: "a-1", vendor: "open_ai", status: "healthy" }),
        createHealth({ account_id: "a-2", vendor: "anthropic", status: "unknown", enabled: false }),
      ]),
    });

    const accounts = useAccounts(client);
    await accounts.load();

    expect(accounts.groups.value).toHaveLength(2);
    expect(accounts.error.value).toBeNull();
  });

  it("creates, updates enabled state, and deletes accounts while refreshing state", async () => {
    const listAccounts = vi
      .fn()
      .mockResolvedValueOnce([createAccount()])
      .mockResolvedValueOnce([
        createAccount(),
        createAccount({
          id: "created-account",
          display_name: "Created Account",
          has_api_key: true,
        }),
      ])
      .mockResolvedValueOnce([
        createAccount({ id: "account-1", enabled: false, display_name: "Disabled Account" }),
        createAccount({
          id: "created-account",
          display_name: "Created Account",
          has_api_key: true,
        }),
      ])
      .mockResolvedValueOnce([
        createAccount({ id: "account-1", enabled: false, display_name: "Disabled Account" }),
      ])
      .mockResolvedValueOnce([
        createAccount({ id: "account-1", enabled: false, display_name: "Disabled Account" }),
      ]);
    const listHealth = vi.fn(async () => [createHealth()]);
    const client = createClient({ listAccounts, listAccountHealth: listHealth });

    const accounts = useAccounts(client);
    await accounts.load();

    await accounts.create({
      vendor: "open_ai",
      display_name: "Created Account",
      api_base_override: null,
      api_key: "sk-created",
      enabled: true,
      weight: 1,
      priority: 0,
      tags: [],
      capabilities: ["chat"],
    });
    await accounts.update(createAccount(), {
      vendor: "open_ai",
      display_name: "Disabled Account",
      api_base_override: null,
      api_key: "",
      enabled: false,
      weight: 1,
      priority: 0,
      tags: ["prod"],
      capabilities: ["chat"],
    });
    await accounts.remove("created-account");

    expect(client.createAccount).toHaveBeenCalledWith(
      expect.objectContaining({ display_name: "Created Account", api_key: "sk-created" })
    );
    expect(client.updateAccount).toHaveBeenCalledWith(
      "account-1",
      expect.objectContaining({ enabled: false })
    );
    expect(client.updateAccount).toHaveBeenCalledWith(
      "account-1",
      expect.not.objectContaining({ api_key: expect.anything() })
    );
    expect(client.deleteAccount).toHaveBeenCalledWith("created-account");
  });

  it("keeps validation failures scoped to the current form action", async () => {
    const accounts = useAccounts(
      createClient({
        createAccount: vi.fn(async () => {
          throw new Error("display_name must not be blank");
        }),
      })
    );

    await accounts.create({
      vendor: "open_ai",
      display_name: "",
      api_base_override: null,
      api_key: null,
      enabled: true,
      weight: 1,
      priority: 0,
      tags: [],
      capabilities: [],
    });

    expect(accounts.actionError.value).toBe("display_name must not be blank");
  });
});
