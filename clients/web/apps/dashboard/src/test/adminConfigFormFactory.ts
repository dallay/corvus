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
};

export function createAdminConfigForm(overrides: Partial<AdminConfigForm> = {}): AdminConfigForm {
  return {
    ...defaultAdminConfigForm,
    ...overrides,
  };
}
