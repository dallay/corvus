export type SecretMode = "unchanged" | "replace" | "clear";

export interface SecretUpdate {
  mode: SecretMode;
  value?: string;
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
