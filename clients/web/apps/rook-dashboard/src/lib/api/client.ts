import { trimTrailingSlashes } from "@corvus/shared";

import type {
  AccountView,
  AddPoolMemberRequest,
  AdminErrorPayload,
  CreateAccountRequest,
  CreatePoolRequest,
  CreateRouteRequest,
  HealthAccountView,
  HealthSummaryView,
  PoolView,
  RouteView,
  SettingsView,
  UpdateAccountRequest,
  UpdatePoolRequest,
  UpdateRouteRequest,
  UpdateSettingsRequest,
  UsageStatusView,
} from "./types";

export interface RookApi {
  listAccounts(): Promise<AccountView[]>;
  getAccount(accountId: string): Promise<AccountView>;
  listPools(): Promise<PoolView[]>;
  getPool(poolId: string): Promise<PoolView>;
  createPool(payload: CreatePoolRequest): Promise<PoolView>;
  updatePool(poolId: string, payload: UpdatePoolRequest): Promise<PoolView>;
  deletePool(poolId: string): Promise<void>;
  addPoolMember(poolId: string, payload: AddPoolMemberRequest): Promise<PoolView>;
  removePoolMember(poolId: string, accountId: string): Promise<PoolView>;
  listRoutes(): Promise<RouteView[]>;
  getRoute(routeId: string): Promise<RouteView>;
  createRoute(payload: CreateRouteRequest): Promise<RouteView>;
  updateRoute(routeId: string, payload: UpdateRouteRequest): Promise<RouteView>;
  deleteRoute(routeId: string): Promise<void>;
  listAccountHealth(): Promise<HealthAccountView[]>;
  getHealthSummary(): Promise<HealthSummaryView>;
  getUsage(): Promise<UsageStatusView>;
  getSettings(): Promise<SettingsView>;
  updateSettings(payload: UpdateSettingsRequest): Promise<SettingsView>;
  createAccount(payload: CreateAccountRequest): Promise<AccountView>;
  updateAccount(accountId: string, payload: UpdateAccountRequest): Promise<AccountView>;
  deleteAccount(accountId: string): Promise<void>;
}

export class RookApiClient implements RookApi {
  private readonly baseUrl: string;
  private readonly bearerToken: string;

  constructor(baseUrl: string, bearerToken: string) {
    const trimmedBaseUrl = trimTrailingSlashes(baseUrl.trim());
    const trimmedToken = bearerToken.trim();

    if (!trimmedBaseUrl) {
      throw new TypeError("baseUrl is required and cannot be empty");
    }

    if (!trimmedToken) {
      throw new TypeError("bearerToken is required and cannot be empty");
    }

    this.baseUrl = trimmedBaseUrl;
    this.bearerToken = trimmedToken;
  }

  listAccounts(): Promise<AccountView[]> {
    return this.request<AccountView[]>("/api/accounts");
  }

  getAccount(accountId: string): Promise<AccountView> {
    return this.request<AccountView>(`/api/accounts/${encodeURIComponent(accountId)}`);
  }

  listPools(): Promise<PoolView[]> {
    return this.request<PoolView[]>("/api/pools");
  }

  getPool(poolId: string): Promise<PoolView> {
    return this.request<PoolView>(`/api/pools/${encodeURIComponent(poolId)}`);
  }

  createPool(payload: CreatePoolRequest): Promise<PoolView> {
    return this.request<PoolView>("/api/pools", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }

  updatePool(poolId: string, payload: UpdatePoolRequest): Promise<PoolView> {
    return this.request<PoolView>(`/api/pools/${encodeURIComponent(poolId)}`, {
      method: "PUT",
      body: JSON.stringify(payload),
    });
  }

  deletePool(poolId: string): Promise<void> {
    return this.request<void>(`/api/pools/${encodeURIComponent(poolId)}`, {
      method: "DELETE",
    });
  }

  addPoolMember(poolId: string, payload: AddPoolMemberRequest): Promise<PoolView> {
    return this.request<PoolView>(`/api/pools/${encodeURIComponent(poolId)}/accounts`, {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }

  removePoolMember(poolId: string, accountId: string): Promise<PoolView> {
    return this.request<PoolView>(
      `/api/pools/${encodeURIComponent(poolId)}/accounts/${encodeURIComponent(accountId)}`,
      {
        method: "DELETE",
      }
    );
  }

  listRoutes(): Promise<RouteView[]> {
    return this.request<RouteView[]>("/api/routes");
  }

  getRoute(routeId: string): Promise<RouteView> {
    return this.request<RouteView>(`/api/routes/${encodeURIComponent(routeId)}`);
  }

  createRoute(payload: CreateRouteRequest): Promise<RouteView> {
    return this.request<RouteView>("/api/routes", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }

  updateRoute(routeId: string, payload: UpdateRouteRequest): Promise<RouteView> {
    return this.request<RouteView>(`/api/routes/${encodeURIComponent(routeId)}`, {
      method: "PUT",
      body: JSON.stringify(payload),
    });
  }

  deleteRoute(routeId: string): Promise<void> {
    return this.request<void>(`/api/routes/${encodeURIComponent(routeId)}`, {
      method: "DELETE",
    });
  }

  listAccountHealth(): Promise<HealthAccountView[]> {
    return this.request<HealthAccountView[]>("/api/health/accounts");
  }

  getHealthSummary(): Promise<HealthSummaryView> {
    return this.request<HealthSummaryView>("/api/health/summary");
  }

  getUsage(): Promise<UsageStatusView> {
    return this.request<UsageStatusView>("/api/usage");
  }

  getSettings(): Promise<SettingsView> {
    return this.request<SettingsView>("/api/settings");
  }

  updateSettings(payload: UpdateSettingsRequest): Promise<SettingsView> {
    return this.request<SettingsView>("/api/settings", {
      method: "PUT",
      body: JSON.stringify(payload),
    });
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
    const headers = new Headers(init?.headers);
    headers.set("Authorization", `Bearer ${this.bearerToken}`);

    if (typeof init?.body === "string" && !headers.has("Content-Type")) {
      headers.set("Content-Type", "application/json");
    }

    const response = await fetch(`${this.baseUrl}${path}`, {
      ...init,
      headers,
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
