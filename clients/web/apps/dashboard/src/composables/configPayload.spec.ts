import { describe, expect, it } from "vitest";

import { buildPayloadForSection } from "@/composables/configPayload";
import type { AdminConfigForm, AdminConfigSnapshot } from "@/types/admin-config";

function createSnapshot(): AdminConfigSnapshot {
  return {
    default_provider: "openai",
    default_model: "gpt-4.1",
    api_url: "http://localhost:3000/api",
    default_temperature: 0.7,
    memory_backend: "sqlite",
    observability_backend: "none",
    otel_endpoint: "",
    otel_service_name: "",
    runtime_kind: "native",
    autonomy_level: "supervised",
    autonomy_workspace_only: true,
    autonomy_max_actions_per_hour: 20,
    autonomy_max_cost_per_day_cents: 500,
    identity_format: "openclaw",
    identity_aieos_path: "",
    scheduler_enabled: true,
    scheduler_max_tasks: 64,
    scheduler_max_concurrent: 4,
    gateway_port: 3000,
    gateway_host: "127.0.0.1",
    gateway_require_pairing: true,
    gateway_allow_public_bind: false,
    gateway_pair_rate_limit_per_minute: 10,
    gateway_webhook_rate_limit_per_minute: 60,
    webhook_enabled: false,
    webhook_port: 3001,
    webhook_secret_exists: false,
  };
}

function createForm(overrides: Partial<AdminConfigForm> = {}): AdminConfigForm {
  const snapshot = createSnapshot();
  return {
    default_provider: snapshot.default_provider,
    default_model: snapshot.default_model,
    api_url: snapshot.api_url,
    default_temperature: `${snapshot.default_temperature}`,
    memory_backend: snapshot.memory_backend,
    observability_backend: snapshot.observability_backend,
    otel_endpoint: snapshot.otel_endpoint,
    otel_service_name: snapshot.otel_service_name,
    runtime_kind: snapshot.runtime_kind,
    autonomy_level: snapshot.autonomy_level,
    autonomy_workspace_only: snapshot.autonomy_workspace_only,
    autonomy_max_actions_per_hour: `${snapshot.autonomy_max_actions_per_hour}`,
    autonomy_max_cost_per_day_cents: `${snapshot.autonomy_max_cost_per_day_cents}`,
    identity_format: snapshot.identity_format,
    identity_aieos_path: snapshot.identity_aieos_path,
    scheduler_enabled: snapshot.scheduler_enabled,
    scheduler_max_tasks: `${snapshot.scheduler_max_tasks}`,
    scheduler_max_concurrent: `${snapshot.scheduler_max_concurrent}`,
    gateway_port: `${snapshot.gateway_port}`,
    gateway_host: snapshot.gateway_host,
    gateway_require_pairing: snapshot.gateway_require_pairing,
    gateway_allow_public_bind: snapshot.gateway_allow_public_bind,
    gateway_pair_rate_limit_per_minute: `${snapshot.gateway_pair_rate_limit_per_minute}`,
    gateway_webhook_rate_limit_per_minute: `${snapshot.gateway_webhook_rate_limit_per_minute}`,
    webhook_enabled: snapshot.webhook_enabled,
    webhook_port: `${snapshot.webhook_port}`,
    webhook_secret_mode: "unchanged",
    webhook_secret_value: "",
    webhook_secret_exists: snapshot.webhook_secret_exists,
    ...overrides,
  };
}

describe("buildPayloadForSection", () => {
  it("builds diff-only payloads for general settings", () => {
    const snapshot = createSnapshot();
    const payload = buildPayloadForSection(
      "general",
      createForm({
        default_provider: "anthropic",
        default_temperature: "not-a-number",
        memory_backend: "surreal",
      }),
      snapshot
    );

    expect(payload).toEqual({
      default_provider: "anthropic",
      memory_backend: "surreal",
    });
  });

  it("nests changed security fields and ignores unchanged values", () => {
    const snapshot = createSnapshot();
    const payload = buildPayloadForSection(
      "security",
      createForm({
        autonomy_level: "full",
        autonomy_workspace_only: false,
        autonomy_max_actions_per_hour: "30",
        autonomy_max_cost_per_day_cents: "",
        identity_aieos_path: "/tmp/identity.json",
      }),
      snapshot
    );

    expect(payload).toEqual({
      autonomy: {
        level: "full",
        workspace_only: false,
        max_actions_per_hour: 30,
      },
      identity: {
        aieos_path: "/tmp/identity.json",
      },
    });
  });

  it("builds observability, runtime, scheduler, and gateway sections only when changed", () => {
    const snapshot = createSnapshot();

    expect(
      buildPayloadForSection(
        "observability",
        createForm({
          observability_backend: "otel",
          otel_endpoint: "http://localhost:4318",
          otel_service_name: "dashboard",
        }),
        snapshot
      )
    ).toEqual({
      observability: {
        backend: "otel",
        otel_endpoint: "http://localhost:4318",
        otel_service_name: "dashboard",
      },
    });

    expect(buildPayloadForSection("runtime", createForm(), snapshot)).toEqual({});
    expect(
      buildPayloadForSection("runtime", createForm({ runtime_kind: "docker" }), snapshot)
    ).toEqual({ runtime: { kind: "docker" } });

    expect(
      buildPayloadForSection(
        "scheduler",
        createForm({ scheduler_enabled: false, scheduler_max_tasks: "96" }),
        snapshot
      )
    ).toEqual({
      scheduler: {
        enabled: false,
        max_tasks: 96,
      },
    });

    expect(
      buildPayloadForSection(
        "gateway",
        createForm({
          gateway_port: "4000",
          gateway_allow_public_bind: true,
          gateway_pair_rate_limit_per_minute: "",
        }),
        snapshot
      )
    ).toEqual({
      gateway: {
        port: 4000,
        allow_public_bind: true,
      },
    });
  });

  it("supports webhook secret clear and replace flows", () => {
    const snapshot = createSnapshot();

    expect(
      buildPayloadForSection(
        "webhook",
        createForm({ webhook_enabled: true, webhook_secret_mode: "clear" }),
        snapshot
      )
    ).toEqual({
      channels: {
        webhook: {
          enabled: true,
          secret: { mode: "clear" },
        },
      },
    });

    expect(
      buildPayloadForSection(
        "webhook",
        createForm({
          webhook_secret_mode: "replace",
          webhook_secret_value: "  top-secret  ",
        }),
        snapshot
      )
    ).toEqual({
      channels: {
        webhook: {
          secret: { mode: "replace", value: "top-secret" },
        },
      },
    });
  });

  it("throws when replacing webhook secret with an empty value", () => {
    expect(() =>
      buildPayloadForSection(
        "webhook",
        createForm({ webhook_secret_mode: "replace", webhook_secret_value: "   " }),
        createSnapshot()
      )
    ).toThrowError("empty_webhook_secret");
  });
});
