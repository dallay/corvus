export interface AccountView {
  id: string;
  vendor: string;
  display_name: string;
  api_base_override: string | null;
  has_api_key: boolean;
  enabled: boolean;
  weight: number;
  priority: number;
  tags: string[];
  capabilities: string[];
}

export interface HealthAccountView {
  account_id: string;
  display_name: string;
  vendor: string;
  enabled: boolean;
  status: "healthy" | "degraded" | "unhealthy" | "unknown";
  last_checked: string | null;
  consecutive_failures: number;
  cooldown_until: string | null;
  is_available: boolean;
}

export interface HealthSummaryView {
  total: number;
  healthy: number;
  degraded: number;
  unhealthy: number;
  unknown: number;
}

export interface CreateAccountRequest {
  vendor: string;
  display_name: string;
  api_base_override?: string | null;
  api_key?: string | null;
  enabled?: boolean;
  weight?: number;
  priority?: number;
  tags?: string[];
  capabilities?: string[];
}

export interface UpdateAccountRequest {
  vendor: string;
  display_name: string;
  api_base_override?: string | null;
  api_key?: string;
  enabled: boolean;
  weight: number;
  priority: number;
  tags: string[];
  capabilities: string[];
}

export interface AdminErrorPayload {
  error?: {
    message?: string;
    code?: string;
  };
}
