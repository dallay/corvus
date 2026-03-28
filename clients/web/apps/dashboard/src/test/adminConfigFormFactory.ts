import type { AdminConfigForm } from "@/types/admin-config";

const defaultAdminConfigForm: AdminConfigForm = {
  default_provider: "",
  default_model: "",
  api_url: "",
  default_temperature: "0.7",
  memory_backend: "sqlite",
  observability_backend: "none",
  otel_endpoint: "",
  otel_service_name: "",
  runtime_kind: "native",
  autonomy_level: "supervised",
  autonomy_workspace_only: true,
  autonomy_max_actions_per_hour: "20",
  autonomy_max_cost_per_day_cents: "500",
  autonomy_require_approval_for_medium_risk: true,
  autonomy_block_high_risk_commands: true,
  autonomy_auto_approve: "",
  autonomy_always_ask: "",
  identity_format: "openclaw",
  identity_aieos_path: "",
  scheduler_enabled: true,
  scheduler_max_tasks: "64",
  scheduler_max_concurrent: "4",
  gateway_port: "3000",
  gateway_host: "127.0.0.1",
  gateway_require_pairing: true,
  gateway_allow_public_bind: false,
  gateway_pair_rate_limit_per_minute: "10",
  gateway_webhook_rate_limit_per_minute: "60",
  webhook_enabled: false,
  webhook_port: "3001",
  webhook_secret_mode: "unchanged",
  webhook_secret_value: "",
  webhook_secret_exists: false,
  // Web Search
  web_search_enabled: false,
  web_search_provider: "duckduckgo",
  web_search_max_results: "5",
  web_search_timeout_secs: "10",
  web_search_brave_api_key_mode: "unchanged",
  web_search_brave_api_key_value: "",
  web_search_has_brave_api_key: false,
  // Browser
  browser_computer_use_api_key_mode: "unchanged",
  browser_computer_use_api_key_value: "",
  browser_has_computer_use_api_key: false,
  // Composio
  composio_enabled: false,
  composio_entity_id: "default",
  composio_api_key_mode: "unchanged",
  composio_api_key_value: "",
  composio_has_api_key: false,
  // Memory (extended)
  memory_cerebro_endpoint: "",
  memory_cerebro_timeout_ms: "5000",
  memory_cerebro_allow_insecure_loopback: false,
  memory_cerebro_auth_token_mode: "unchanged",
  memory_cerebro_auth_token_value: "",
  memory_cerebro_has_auth_token: false,
};

export function createAdminConfigForm(overrides: Partial<AdminConfigForm> = {}): AdminConfigForm {
  return {
    ...defaultAdminConfigForm,
    ...overrides,
  };
}
