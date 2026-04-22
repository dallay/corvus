import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type {
  AccountView,
  CreateAccountRequest,
  HealthAccountView,
  UpdateAccountRequest,
} from "@/lib/api/types";

import AccountsPage from "./AccountsPage.vue";

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
    listAccounts: vi.fn(async () => [createAccount()]),
    getAccount: vi.fn(async (accountId: string) => createAccount({ id: accountId })),
    listAccountHealth: vi.fn(async () => [createHealth()]),
    getHealthSummary: vi.fn(),
    createAccount: vi.fn(async (payload: CreateAccountRequest) =>
      createAccount({
        id: "created-account",
        vendor: payload.vendor,
        display_name: payload.display_name,
        has_api_key: Boolean(payload.api_key),
        enabled: payload.enabled ?? true,
      })
    ),
    updateAccount: vi.fn(async (accountId: string, payload: UpdateAccountRequest) =>
      createAccount({
        id: accountId,
        vendor: payload.vendor,
        display_name: payload.display_name,
        enabled: payload.enabled,
      })
    ),
    deleteAccount: vi.fn(async () => undefined),
    ...overrides,
  } as unknown as RookApiClient;
}

async function mountPage(client: RookApiClient) {
  const wrapper = mount(AccountsPage, {
    props: { client },
    attachTo: document.body,
  });

  await vi.waitFor(() => {
    expect(wrapper.text()).toContain("Primary OpenAI");
  });
  await flushPromises();
  return wrapper;
}

describe("AccountsPage", () => {
  it("opens account detail from the grouped account list", async () => {
    const client = createClient();
    const wrapper = await mountPage(client);

    await wrapper.get(".link-button").trigger("click");
    await flushPromises();

    expect(client.getAccount).toHaveBeenCalledWith("account-1");
    expect(wrapper.text()).toContain("Account detail");
    expect(wrapper.text()).toContain("Stored API key exists");
  });

  it("shows account list loading state while requests are pending", async () => {
    let resolveAccounts: ((value: AccountView[]) => void) | undefined;
    const client = createClient({
      listAccounts: vi.fn(
        () =>
          new Promise<AccountView[]>((resolve) => {
            resolveAccounts = resolve;
          })
      ),
    });

    const wrapper = mount(AccountsPage, {
      props: { client },
      attachTo: document.body,
    });

    await Promise.resolve();
    expect(wrapper.text()).toContain("Loading provider accounts…");

    resolveAccounts?.([createAccount()]);
    await flushPromises();
  });

  it("creates a disabled account with existing enabled semantics", async () => {
    const client = createClient();
    const wrapper = await mountPage(client);

    await wrapper.get(".primary-button").trigger("click");
    await wrapper.get('input[name="display_name"]').setValue("Disabled Account");
    await wrapper.get('input[name="api_key"]').setValue("sk-disabled");
    await wrapper.get('input[name="enabled"]').setValue(false);
    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(client.createAccount).toHaveBeenCalledWith(
      expect.objectContaining({
        display_name: "Disabled Account",
        enabled: false,
      })
    );
  });

  it("re-enables an existing account without showing unsupported connection testing", async () => {
    const client = createClient({
      getAccount: vi.fn(async () => createAccount({ enabled: false, has_api_key: true })),
    });
    const wrapper = await mountPage(client);

    await wrapper.get(".row-actions button").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain(
      "Stored API key exists. Leave the replacement field blank to preserve it."
    );
    expect(wrapper.text()).not.toContain("Test connection");

    await wrapper.get('input[name="enabled"]').setValue(true);
    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(client.updateAccount).toHaveBeenCalledWith(
      "account-1",
      expect.objectContaining({ enabled: true })
    );
    expect(client.updateAccount).toHaveBeenCalledWith(
      "account-1",
      expect.not.objectContaining({ api_key: expect.anything() })
    );
  });

  it("surfaces delete conflict without removing the account", async () => {
    const client = createClient({
      deleteAccount: vi.fn(async () => {
        throw new Error("account is referenced by a pool");
      }),
    });
    const wrapper = await mountPage(client);

    await wrapper.findAll(".row-actions button")[1]?.trigger("click");
    await wrapper.get(".danger-button").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("account is referenced by a pool");
    expect(wrapper.text()).toContain("Primary OpenAI");
  });
});
