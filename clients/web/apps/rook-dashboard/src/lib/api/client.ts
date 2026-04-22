import { trimTrailingSlashes } from "@corvus/shared";

import type {
  AccountView,
  AdminErrorPayload,
  CreateAccountRequest,
  HealthAccountView,
  HealthSummaryView,
  UpdateAccountRequest,
} from "./types";

export interface RookApi {
  listAccounts(): Promise<AccountView[]>;
  getAccount(accountId: string): Promise<AccountView>;
  listAccountHealth(): Promise<HealthAccountView[]>;
  getHealthSummary(): Promise<HealthSummaryView>;
  createAccount(payload: CreateAccountRequest): Promise<AccountView>;
  updateAccount(accountId: string, payload: UpdateAccountRequest): Promise<AccountView>;
  deleteAccount(accountId: string): Promise<void>;
}

export class RookApiClient implements RookApi {
  constructor(
    private readonly baseUrl: string,
    private readonly bearerToken: string
  ) {}

  listAccounts(): Promise<AccountView[]> {
    return this.request<AccountView[]>("/api/accounts");
  }

  getAccount(accountId: string): Promise<AccountView> {
    return this.request<AccountView>(`/api/accounts/${encodeURIComponent(accountId)}`);
  }

  listAccountHealth(): Promise<HealthAccountView[]> {
    return this.request<HealthAccountView[]>("/api/health/accounts");
  }

  getHealthSummary(): Promise<HealthSummaryView> {
    return this.request<HealthSummaryView>("/api/health/summary");
  }

  createAccount(payload: CreateAccountRequest): Promise<AccountView> {
    return this.request<AccountView>("/api/accounts", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }

  updateAccount(accountId: string, payload: UpdateAccountRequest): Promise<AccountView> {
    return this.request<AccountView>(`/api/accounts/${encodeURIComponent(accountId)}`, {
      method: "PUT",
      body: JSON.stringify(payload),
    });
  }

  deleteAccount(accountId: string): Promise<void> {
    return this.request<void>(`/api/accounts/${encodeURIComponent(accountId)}`, {
      method: "DELETE",
    });
  }

  private async request<T>(path: string, init?: RequestInit): Promise<T> {
    const response = await fetch(`${trimTrailingSlashes(this.baseUrl)}${path}`, {
      ...init,
      headers: {
        Authorization: `Bearer ${this.bearerToken.trim()}`,
        "Content-Type": "application/json",
        ...(init?.headers ?? {}),
      },
    });

    if (!response.ok) {
      throw new Error(await toErrorMessage(response));
    }

    if (response.status === 204) {
      return undefined as T;
    }

    return (await response.json()) as T;
  }
}

async function toErrorMessage(response: Response): Promise<string> {
  try {
    const json = (await response.json()) as AdminErrorPayload;
    return json.error?.message ?? `HTTP ${response.status}`;
  } catch {
    return `HTTP ${response.status}`;
  }
}
