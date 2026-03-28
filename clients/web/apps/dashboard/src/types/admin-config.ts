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

export interface AdminWebSearchView {
  enabled: boolean;
  provider: string;
  max_results: number;
  timeout_secs: number;
  has_brave_api_key: boolean;
}

export interface AdminBrowserView {
  has_computer_use_api_key: boolean;
}

export interface AdminComposioView {
  enabled: boolean;
  entity_id: string;
  has_api_key: boolean;
}

export interface AdminMemoryView {
  backend: string;
  cerebro: AdminCerebroMemoryView;
}

export interface AdminCerebroMemoryView {
  endpoint?: string | null;
  has_auth_token: boolean;
  request_timeout_ms: number;
  allow_insecure_loopback: boolean;
}

export interface AdminUpdatesView {
  enabled: boolean;
  auto_install_enabled: boolean;
  channel_visibility_enabled: boolean;
  cli_startup_notice_enabled: boolean;
  install_method_override?: string | null;
  restart_policy: string;
  status: AdminUpdateStatusView;
}

export interface AdminUpdateStatusView {
  current_version: string;
  latest_version?: string | null;
  update_available: boolean;
  last_check_at_unix?: number | null;
  last_check_outcome?: string | null;
  effective_install_method: string;
  install_method_source: string;
}

export interface AdminProviderView {
  has_api_key: boolean;
}

export interface AdminIdentityView {
  format: string;
  aieos_path?: string | null;
  has_aieos_inline: boolean;
}

export interface AdminGatewayExtendedView {
  trust_forwarded_headers: boolean;
  rate_limit_max_keys: number;
  idempotency_ttl_secs: number;
  idempotency_max_keys: number;
  paired_tokens_count: number;
}

export interface AdminAutonomyExtendedView {
  require_approval_for_medium_risk: boolean;
  block_high_risk_commands: boolean;
  auto_approve: string[];
  always_ask: string[];
}

export interface AdminHealthSnapshot {
  pid: number;
  updated_at: string;
  uptime_seconds: number;
  components: Record<string, AdminComponentHealth>;
}

export interface AdminComponentHealth {
  status: string;
  updated_at: string;
  last_ok?: string | null;
  last_error?: string | null;
  restart_count: number;
}

export interface AdminSchedulerStatusView {
  enabled: boolean;
  max_tasks: number;
  max_concurrent: number;
  task_count: number;
}

export interface AdminChannelStatusView {
  channel_type: string;
  configured: boolean;
  config_summary: Record<string, unknown>;
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
    require_approval_for_medium_risk?: boolean;
    block_high_risk_commands?: boolean;
    auto_approve?: string[];
    always_ask?: string[];
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
  composio?: AdminComposioView;
  web_search?: AdminWebSearchView;
  memory?: AdminMemoryView;
  browser?: AdminBrowserView;
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
  autonomy_require_approval_for_medium_risk: boolean;
  autonomy_block_high_risk_commands: boolean;
  autonomy_auto_approve: string;
  autonomy_always_ask: string;
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
  // Web Search
  web_search_enabled: boolean;
  web_search_provider: string;
  web_search_max_results: string;
  web_search_timeout_secs: string;
  web_search_brave_api_key_mode: SecretMode;
  web_search_brave_api_key_value: string;
  web_search_has_brave_api_key: boolean;
  // Browser
  browser_computer_use_api_key_mode: SecretMode;
  browser_computer_use_api_key_value: string;
  browser_has_computer_use_api_key: boolean;
  // Composio
  composio_enabled: boolean;
  composio_entity_id: string;
  composio_api_key_mode: SecretMode;
  composio_api_key_value: string;
  composio_has_api_key: boolean;
  // Memory (extended)
  memory_cerebro_endpoint: string;
  memory_cerebro_timeout_ms: string;
  memory_cerebro_allow_insecure_loopback: boolean;
  memory_cerebro_auth_token_mode: SecretMode;
  memory_cerebro_auth_token_value: string;
  memory_cerebro_has_auth_token: boolean;
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
  autonomy_require_approval_for_medium_risk: boolean;
  autonomy_block_high_risk_commands: boolean;
  autonomy_auto_approve: string;
  autonomy_always_ask: string;
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
  // Web Search
  web_search_enabled: boolean;
  web_search_provider: string;
  web_search_max_results: number;
  web_search_timeout_secs: number;
  web_search_has_brave_api_key: boolean;
  // Browser
  browser_has_computer_use_api_key: boolean;
  // Composio
  composio_enabled: boolean;
  composio_entity_id: string;
  composio_has_api_key: boolean;
  // Memory (extended)
  memory_cerebro_endpoint: string;
  memory_cerebro_timeout_ms: number;
  memory_cerebro_allow_insecure_loopback: boolean;
  memory_cerebro_has_auth_token: boolean;
}

export interface AdminComposioPatch {
  enabled?: boolean;
  entity_id?: string;
  api_key?: SecretUpdate;
}

export interface AdminWebSearchPatch {
  enabled?: boolean;
  provider?: string;
  max_results?: number;
  timeout_secs?: number;
  brave_api_key?: SecretUpdate;
}

export interface AdminBrowserPatch {
  computer_use_api_key?: SecretUpdate;
}

export interface AdminMemoryPatch {
  backend?: string;
  cerebro?: AdminCerebroMemoryPatch;
}

export interface AdminCerebroMemoryPatch {
  endpoint?: string;
  request_timeout_ms?: number;
  allow_insecure_loopback?: boolean;
  auth_token?: SecretUpdate;
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
    require_approval_for_medium_risk?: boolean;
    block_high_risk_commands?: boolean;
    auto_approve?: string[];
    always_ask?: string[];
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
  composio?: AdminComposioPatch;
  web_search?: AdminWebSearchPatch;
  browser?: AdminBrowserPatch;
  memory?: AdminMemoryPatch;
}

export type ConfigSection =
  | "general"
  | "security"
  | "observability"
  | "runtime"
  | "scheduler"
  | "gateway"
  | "webhook"
  | "web-search"
  | "browser"
  | "composio"
  | "memory"
  | "provider-pools"
  | "updates";
