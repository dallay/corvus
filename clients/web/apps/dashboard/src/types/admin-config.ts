export type SecretMode = "unchanged" | "replace" | "clear";

export interface SecretUpdate {
  mode: SecretMode;
  value?: string;
}

export type AccountPoolStrategy = "round_robin" | "weighted_round_robin";

export interface ProviderAccountConfig {
  id: string;
  api_key: string;
  api_url?: string | null;
  weight?: number;
  enabled?: boolean;
}

export interface ProviderAccountPoolConfig {
  strategy?: AccountPoolStrategy;
  accounts: ProviderAccountConfig[];
}

export interface AdminProviderAccountView {
  id: string;
  api_url?: string | null;
  weight: number;
  enabled: boolean;
  has_api_key: boolean;
}

export interface AdminProviderPoolView {
  strategy: AccountPoolStrategy;
  accounts: AdminProviderAccountView[];
}

export interface AdminProviderPoolsView {
  account_pools: Record<string, AdminProviderPoolView>;
}

export interface AdminProviderPoolsResponse {
  pools: AdminProviderPoolsView;
}

export interface AdminProviderPoolsUpdateRequest {
  account_pools: Record<string, ProviderAccountPoolConfig>;
}

export interface AdminProviderPoolsUpdateResponse {
  updated: boolean;
  pools: AdminProviderPoolsView;
}

export interface AdminOptionsResponse {
  memory_backends?: string[];
  observability_backends?: string[];
  runtime_kinds?: string[];
  autonomy_levels?: string[];
}

export interface AdminConfigView {
  default_provider?: string | null;
  default_model?: string | null;
  api_url?: string | null;
  default_temperature?: number;
  memory_backend?: string;
  provider?: {
    has_api_key?: boolean;
  };
  observability?: {
    backend?: string;
    otel_endpoint?: string | null;
    otel_service_name?: string | null;
  };
  runtime?: {
    kind?: string;
  };
  autonomy?: {
    level?: string;
    workspace_only?: boolean;
    max_actions_per_hour?: number;
    max_cost_per_day_cents?: number;
  };
  identity?: {
    format?: string;
    aieos_path?: string | null;
    has_aieos_inline?: boolean;
  };
  scheduler?: {
    enabled?: boolean;
    max_tasks?: number;
    max_concurrent?: number;
  };
  gateway?: {
    port?: number;
    host?: string;
    require_pairing?: boolean;
    allow_public_bind?: boolean;
    pair_rate_limit_per_minute?: number;
    webhook_rate_limit_per_minute?: number;
  };
  channels?: {
    webhook?: {
      enabled?: boolean;
      port?: number;
      has_secret?: boolean;
    };
  };
  updates?: {
    enabled?: boolean;
    auto_install_enabled?: boolean;
    channel_visibility_enabled?: boolean;
    cli_startup_notice_enabled?: boolean;
    install_method_override?: string | null;
    restart_policy?: string;
    status?: {
      current_version?: string;
      latest_version?: string | null;
      update_available?: boolean;
      last_check_at_unix?: number | null;
      last_check_outcome?: string | null;
      effective_install_method?: string;
      install_method_source?: string;
    };
  };
}

export interface AdminConfigResponse {
  config?: AdminConfigView;
}

export interface AdminConfigForm {
  default_provider: string;
  default_model: string;
  api_url: string;
  default_temperature: string;
  memory_backend: string;
  observability_backend: string;
  otel_endpoint: string;
  otel_service_name: string;
  runtime_kind: string;
  autonomy_level: string;
  autonomy_workspace_only: boolean;
  autonomy_max_actions_per_hour: string;
  autonomy_max_cost_per_day_cents: string;
  identity_format: string;
  identity_aieos_path: string;
  scheduler_enabled: boolean;
  scheduler_max_tasks: string;
  scheduler_max_concurrent: string;
  gateway_port: string;
  gateway_host: string;
  gateway_require_pairing: boolean;
  gateway_allow_public_bind: boolean;
  gateway_pair_rate_limit_per_minute: string;
  gateway_webhook_rate_limit_per_minute: string;
  webhook_enabled: boolean;
  webhook_port: string;
  webhook_secret_mode: SecretMode;
  webhook_secret_value: string;
  webhook_secret_exists: boolean;
}

export interface AdminConfigSnapshot {
  default_provider: string;
  default_model: string;
  api_url: string;
  default_temperature: number;
  memory_backend: string;
  observability_backend: string;
  otel_endpoint: string;
  otel_service_name: string;
  runtime_kind: string;
  autonomy_level: string;
  autonomy_workspace_only: boolean;
  autonomy_max_actions_per_hour: number;
  autonomy_max_cost_per_day_cents: number;
  identity_format: string;
  identity_aieos_path: string;
  scheduler_enabled: boolean;
  scheduler_max_tasks: number;
  scheduler_max_concurrent: number;
  gateway_port: number;
  gateway_host: string;
  gateway_require_pairing: boolean;
  gateway_allow_public_bind: boolean;
  gateway_pair_rate_limit_per_minute: number;
  gateway_webhook_rate_limit_per_minute: number;
  webhook_enabled: boolean;
  webhook_port: number;
  webhook_secret_exists: boolean;
}

export interface AdminConfigUpdateRequest {
  default_provider?: string;
  default_model?: string;
  api_url?: string;
  default_temperature?: number;
  memory_backend?: string;
  provider?: {
    api_key?: SecretUpdate;
  };
  observability?: {
    backend?: string;
    otel_endpoint?: string;
    otel_service_name?: string;
  };
  runtime?: {
    kind?: string;
  };
  autonomy?: {
    level?: string;
    workspace_only?: boolean;
    max_actions_per_hour?: number;
    max_cost_per_day_cents?: number;
  };
  identity?: {
    format?: string;
    aieos_path?: string;
  };
  scheduler?: {
    enabled?: boolean;
    max_tasks?: number;
    max_concurrent?: number;
  };
  gateway?: {
    port?: number;
    host?: string;
    require_pairing?: boolean;
    allow_public_bind?: boolean;
    pair_rate_limit_per_minute?: number;
    webhook_rate_limit_per_minute?: number;
  };
  channels?: {
    webhook?: {
      enabled?: boolean;
      port?: number;
      secret?: SecretUpdate;
    };
  };
}

export type ConfigSection =
  | "general"
  | "security"
  | "observability"
  | "runtime"
  | "scheduler"
  | "gateway"
  | "webhook";
