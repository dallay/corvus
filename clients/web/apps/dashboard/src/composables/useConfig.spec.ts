import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  dashboardOnboardingRecoveryLabel,
  dashboardOnboardingTransitionLabel,
  useConfig,
} from "@/composables/useConfig";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

describe("useConfig", () => {
  it("maps initial fetch response and options", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ memory_backends: ["sqlite", "none"] }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            config: {
              default_provider: "openrouter",
              default_model: "anthropic/claude-sonnet-4",
              default_temperature: 0.7,
              memory_backend: "sqlite",
              autonomy: {
                level: "supervised",
                workspace_only: true,
                max_actions_per_hour: 20,
                max_cost_per_day_cents: 500,
              },
              channels: { webhook: { enabled: true, port: 3010, has_secret: true } },
            },
          }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }
        )
      );

    const config = useConfig((key: string) => key);
    config.bearerToken.value = "token";
    await config.connectGateway();

    expect(config.form.default_provider).toBe("openrouter");
    expect(config.form.webhook_secret_exists).toBe(true);
    expect(config.memoryBackendOptions.value).toEqual(["sqlite", "none"]);
    expect(config.onboardingState.value.state).toBe("ready");
  });

  it("maps invalid pairing input to the normalized trust recovery state", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ error: "Invalid pairing code" }), {
        status: 403,
        headers: { "Content-Type": "application/json" },
      })
    );

    const config = useConfig((key: string) => key);
    config.pairingCode.value = "wrong";

    const paired = await config.pairGateway({ autoConnect: false });

    expect(paired).toBe(false);
    expect(config.onboardingState.value.state).toBe("blocked");
    expect(config.onboardingState.value.recoveryKind).toBe("trust_input_invalid");
    expect(config.lastTransitionLabel.value).toBe("runtime_path_confirmed__to__blocked");
    expect(config.currentRecoveryLabel.value).toBe("trust_input_invalid");
    expect(config.errorMessage.value).toBe("auth.pairingInvalid");
    expect(config.pairingCode.value).toBe("wrong");
    expect(config.bearerToken.value).toBe("");
  });

  it("exposes canonical observability labels for onboarding transitions and recovery", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ token: "token-123" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );

    const config = useConfig((key: string) => key);
    config.pairingCode.value = "857258";

    const paired = await config.pairGateway({ autoConnect: false });

    expect(paired).toBe(true);
    expect(config.lastTransitionLabel.value).toBe("trust_pending__to__trust_established");
    expect(config.currentRecoveryLabel.value).toBe(null);
    expect(dashboardOnboardingTransitionLabel("trust_pending", "trust_established")).toBe(
      "trust_pending__to__trust_established"
    );
    expect(dashboardOnboardingRecoveryLabel("paired_but_not_connected")).toBe(
      "paired_but_not_connected"
    );
  });

  it("maps expired pairing input to the normalized trust recovery state", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ error: "Pairing code expired" }), {
        status: 410,
        headers: { "Content-Type": "application/json" },
      })
    );

    const config = useConfig((key: string) => key);
    config.pairingCode.value = "stale";

    const paired = await config.pairGateway({ autoConnect: false });

    expect(paired).toBe(false);
    expect(config.onboardingState.value.state).toBe("blocked");
    expect(config.onboardingState.value.recoveryKind).toBe("trust_input_expired");
    expect(config.errorMessage.value).toBe("auth.pairingExpired");
  });

  it("clears stale bearer tokens when gateway auth rejects them", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ error: "Unauthorized" }), {
        status: 401,
        headers: { "Content-Type": "application/json" },
      })
    );

    const config = useConfig((key: string) => key);
    config.bearerToken.value = "stale-token";

    const connected = await config.connectGateway();

    expect(connected).toBe(false);
    expect(config.bearerToken.value).toBe("");
    expect(config.onboardingState.value.state).toBe("blocked");
    expect(config.onboardingState.value.recoveryKind).toBe("credential_invalid");
    expect(config.errorMessage.value).toBe("auth.credentialInvalid");
  });

  it("marks missing bearer tokens as credential_missing when admin access is rejected", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ error: "Unauthorized" }), {
        status: 401,
        headers: { "Content-Type": "application/json" },
      })
    );

    const config = useConfig((key: string) => key);

    const connected = await config.connectGateway();

    expect(connected).toBe(false);
    expect(config.onboardingState.value.state).toBe("blocked");
    expect(config.onboardingState.value.recoveryKind).toBe("credential_missing");
    expect(config.errorMessage.value).toBe("auth.credentialMissing");
  });

  it("preserves trust progress when the dashboard is paired but the gateway cannot complete auth fetches", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ token: "token-123" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 503 }));

    const config = useConfig((key: string) => key);
    config.pairingCode.value = "857258";

    const paired = await config.pairGateway();

    expect(paired).toBe(false);
    expect(config.bearerToken.value).toBe("token-123");
    expect(config.onboardingState.value.state).toBe("blocked");
    expect(config.onboardingState.value.recoveryKind).toBe("paired_but_not_connected");
    expect(config.onboardingState.value.canResume).toBe(true);
    expect(config.onboardingSteps.value[1]?.status).toBe("complete");
    expect(config.onboardingSteps.value[2]?.status).toBe("blocked");
  });

  it("tracks section saving and sends diff-only payload", async () => {
    fetchMock
      .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({ config: { default_model: "a", channels: { webhook: {} } } }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }
        )
      )
      .mockResolvedValueOnce(new Response(JSON.stringify({ updated: true }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({ config: { default_model: "b", channels: { webhook: {} } } }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }
        )
      );

    const config = useConfig((key: string) => key);
    config.bearerToken.value = "token";
    await config.connectGateway();
    config.form.default_model = "b";

    const promise = config.saveSection("general");
    expect(config.sectionSaving.general).toBe(true);
    await promise;
    expect(config.sectionSaving.general).toBe(false);

    const saveCall = fetchMock.mock.calls.find((entry) => (entry[1]?.method ?? "GET") === "PUT");
    const body = JSON.parse((saveCall?.[1]?.body as string) || "{}");
    expect(body.default_model).toBe("b");
    expect(body.default_provider).toBeUndefined();
  });

  it("rejects replace secret mode with empty value", async () => {
    fetchMock
      .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ config: { channels: { webhook: {} } } }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

    const config = useConfig((key: string) => key);
    config.bearerToken.value = "token";
    await config.connectGateway();
    config.form.webhook_secret_mode = "replace";
    config.form.webhook_secret_value = "  ";

    await config.saveSection("webhook");

    expect(config.errorMessage.value).toBe("auth.emptyWebhookSecret");
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("auto-connects after pairing with same-origin proxied api endpoints by default", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ token: "token-123" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ config: { channels: { webhook: {} } } }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

    const config = useConfig((key: string) => key);
    config.pairingCode.value = "857258";

    await config.pairGateway();

    expect(fetchMock.mock.calls[0]?.[0]).toBe("http://localhost:3000/api/pair");
    expect(fetchMock.mock.calls[1]?.[0]).toBe("http://localhost:3000/api/web/admin/options");
    expect(fetchMock.mock.calls[2]?.[0]).toBe("http://localhost:3000/api/web/admin/config");
  });

  it("preserves absolute base paths when auto-connecting after pairing", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ token: "token-123" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ config: { channels: { webhook: {} } } }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

    const config = useConfig((key: string) => key);
    config.baseUrl.value = "http://corvus.localhost/api";
    config.pairingCode.value = "857258";

    await config.pairGateway();

    expect(fetchMock.mock.calls[0]?.[0]).toBe("http://corvus.localhost/api/pair");
    expect(fetchMock.mock.calls[1]?.[0]).toBe("http://corvus.localhost/api/web/admin/options");
    expect(fetchMock.mock.calls[2]?.[0]).toBe("http://corvus.localhost/api/web/admin/config");
  });

  it("normalizes repeated slashes in configured gateway URLs", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ token: "token-123" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ config: { channels: { webhook: {} } } }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

    const config = useConfig((key: string) => key);
    config.baseUrl.value = "http://corvus.localhost/api///";
    config.pairingCode.value = "857258";

    await config.pairGateway();

    expect(fetchMock.mock.calls[0]?.[0]).toBe("http://corvus.localhost/api/pair");
    expect(fetchMock.mock.calls[1]?.[0]).toBe("http://corvus.localhost/api/web/admin/options");
    expect(fetchMock.mock.calls[2]?.[0]).toBe("http://corvus.localhost/api/web/admin/config");
  });

  it("blocks sending secrets to non-local origins", async () => {
    const config = useConfig((key: string) => key);
    config.baseUrl.value = "https://example.com/api";
    config.pairingCode.value = "857258";

    await config.pairGateway();

    expect(config.errorMessage.value).toBe("errors.insecureUrlError");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("allows read-only public config fetches when no bearer token is present", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ observability_backends: ["none", "otel"] }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ config: { channels: { webhook: {} } } }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

    const config = useConfig((key: string) => key);
    config.baseUrl.value = "https://example.com/api";

    await config.connectGateway();

    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[0]?.[1]?.headers).toEqual({
      "Content-Type": "application/json",
    });
    expect(config.observabilityBackendOptions.value).toEqual(["none", "otel"]);
    expect(config.statusMessage.value).toBe("auth.connected");
  });

  it("supports pairing without auto-connect when requested", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ token: "token-123" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );

    const config = useConfig((key: string) => key);
    config.pairingCode.value = "857258";

    const paired = await config.pairGateway({ autoConnect: false });

    expect(paired).toBe(true);
    expect(config.bearerToken.value).toBe("token-123");
    expect(config.statusMessage.value).toBe("auth.pairSuccess");
    expect(config.onboardingState.value.state).toBe("trust_established");
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("reports restart-required conflicts when saving config sections", async () => {
    fetchMock
      .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({ config: { default_model: "a", channels: { webhook: {} } } }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }
        )
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ fields: ["runtime.kind"] }), {
          status: 409,
          headers: { "Content-Type": "application/json" },
        })
      );

    const config = useConfig((key: string, params?: Record<string, unknown>) =>
      params?.fields ? `${key}:${String(params.fields)}` : key
    );
    config.bearerToken.value = "token";
    await config.connectGateway();
    config.form.default_model = "b";

    await config.saveSection("general");

    expect(config.errorMessage.value).toBe("form.restartRequired:runtime.kind");
  });

  it("skips save requests when the selected section has no changes", async () => {
    fetchMock
      .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({ config: { default_model: "a", channels: { webhook: {} } } }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }
        )
      );

    const config = useConfig((key: string) => key);
    config.bearerToken.value = "token";
    await config.connectGateway();

    await config.saveSection("general");

    expect(config.statusMessage.value).toBe("form.noChanges");
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  describe("Quick Pair (Magic Link)", () => {
    let originalLocation: Location;
    let originalHistory: History;

    beforeEach(() => {
      originalLocation = window.location;
      originalHistory = window.history;

      // @ts-expect-error test override
      delete window.location;
      // @ts-expect-error test override
      delete window.history;

      // @ts-expect-error test override
      window.location = {
        hash: "",
        pathname: "/admin",
        search: "",
        href: "http://localhost:3000/admin",
        origin: "http://localhost:3000",
      } as unknown as Location;
      window.history = {
        replaceState: vi.fn(),
      } as unknown as History;
    });

    afterEach(() => {
      // @ts-expect-error test restore
      window.location = originalLocation;
      window.history = originalHistory;
    });

    it("parses valid fragment, scrubs URL, and auto-pairs/connects", async () => {
      window.location.hash = "#/quick-pair?pairingCode=123xyz&gatewayUrl=http://127.0.0.1:4000";

      // Mock for pairGateway
      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify({ token: "magic-token" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );
      // Mocks for connectGateway
      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify({}), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );
      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify({ config: { channels: { webhook: {} } } }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      const config = useConfig((key: string) => key);

      // Should immediately scrub URL
      expect(window.history.replaceState).toHaveBeenCalledWith(null, "", "/admin");

      // Since inner async void is fired, we wait for it to complete
      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(config.baseUrl.value).toBe("http://127.0.0.1:4000");
      expect(config.bearerToken.value).toBe("magic-token");
      expect(config.quickPairState.value).toBe("connected");
      expect(fetchMock).toHaveBeenCalledTimes(3);
      expect(fetchMock.mock.calls[0]?.[0]).toBe("http://127.0.0.1:4000/pair");
    });

    it("rejects non-local / insecure gatewayUrl and shows error", async () => {
      window.location.hash = "#/quick-pair?pairingCode=123xyz&gatewayUrl=https://evil.com/api";

      const config = useConfig((key: string) => key);

      expect(window.history.replaceState).toHaveBeenCalledWith(null, "", "/admin");
      expect(config.quickPairState.value).toBe("failed");
      expect(config.errorMessage.value).toBe("errors.insecureUrlError");
      expect(config.baseUrl.value).toBe("https://evil.com/api");
      expect(config.pairingCode.value).toBe("");
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("rejects gatewayUrl with embedded credentials or query parameters", async () => {
      window.location.hash =
        "#/quick-pair?pairingCode=123xyz&gatewayUrl=http://admin:pass@localhost:3000/?foo=bar";

      const config = useConfig((key: string) => key);

      expect(config.quickPairState.value).toBe("failed");
      expect(config.errorMessage.value).toBe("errors.insecureUrlError");
      expect(config.pairingCode.value).toBe("");
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("gracefully falls back if auto-pair fails", async () => {
      window.location.hash = "#/quick-pair?pairingCode=wrongcode&gatewayUrl=http://localhost:3000";

      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "invalid" }), { status: 401 })
      );

      const config = useConfig((key: string) => key);
      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(config.quickPairState.value).toBe("failed");
      expect(config.errorMessage.value).toBe("auth.pairingInvalid");
      expect(config.pairingCode.value).toBe("");
      expect(config.bearerToken.value).toBe("");
    });
  });

  describe("URL safety checks", () => {
    it("rejects malformed URLs that cannot be parsed", async () => {
      const config = useConfig((key: string) => key);
      config.baseUrl.value = "not-a-url-at-all";
      config.pairingCode.value = "857258";

      const result = await config.pairGateway();

      expect(result).toBe(false);
      expect(config.errorMessage.value).toBe("errors.insecureUrlError");
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("rejects file:// protocol as unsafe", async () => {
      const config = useConfig((key: string) => key);
      config.baseUrl.value = "file:///etc/passwd";
      config.pairingCode.value = "857258";

      const result = await config.pairGateway();

      expect(result).toBe(false);
      expect(config.errorMessage.value).toBe("errors.insecureUrlError");
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("rejects URLs with fragment anchors as unsafe", async () => {
      const config = useConfig((key: string) => key);
      config.baseUrl.value = "http://localhost:3000/#section";
      config.pairingCode.value = "857258";

      const result = await config.pairGateway();

      expect(result).toBe(false);
      expect(config.errorMessage.value).toBe("errors.insecureUrlError");
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("accepts IPv6 loopback address", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify({ token: "ipv6-token" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      const config = useConfig((key: string) => key);
      config.baseUrl.value = "http://[::1]:3000";
      config.pairingCode.value = "857258";

      const result = await config.pairGateway({ autoConnect: false });

      expect(result).toBe(true);
      expect(config.bearerToken.value).toBe("ipv6-token");
    });

    it("accepts subdomains of .localhost as trusted", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify({ token: "subdomain-token" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      const config = useConfig((key: string) => key);
      config.baseUrl.value = "http://corvus.localhost:4000/api";
      config.pairingCode.value = "857258";

      const result = await config.pairGateway({ autoConnect: false });

      expect(result).toBe(true);
      expect(config.bearerToken.value).toBe("subdomain-token");
    });
  });

  describe("saveSection URL safety and error paths", () => {
    it("returns insecureUrlError when baseUrl points to an untrusted origin", async () => {
      // ensure initialConfig is set so we pass the early-return guard
      fetchMock
        .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
        .mockResolvedValueOnce(
          new Response(
            JSON.stringify({
              config: { default_model: "a", channels: { webhook: {} } },
            }),
            { status: 200, headers: { "Content-Type": "application/json" } }
          )
        );

      const config = useConfig((key: string) => key);
      config.bearerToken.value = "token";
      await config.connectGateway();
      const callsBeforeSave = fetchMock.mock.calls.length;
      config.baseUrl.value = "https://malicious.example.com/api";
      config.form.default_model = "b";

      await config.saveSection("general");

      const maliciousCalls = fetchMock.mock.calls.filter(([request]) => {
        const url =
          typeof request === "string" ? request : request instanceof Request ? request.url : "";
        return url.startsWith("https://malicious.example.com/api");
      });

      expect(config.errorMessage.value).toBe("errors.insecureUrlError");
      expect(config.sectionSaving.general).toBe(false);
      expect(maliciousCalls).toHaveLength(0);
      expect(fetchMock.mock.calls.length).toBe(callsBeforeSave);
    });

    it("reports a generic error when save returns a non-OK, non-409 status", async () => {
      fetchMock
        .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
        .mockResolvedValueOnce(
          new Response(
            JSON.stringify({ config: { default_model: "a", channels: { webhook: {} } } }),
            { status: 200, headers: { "Content-Type": "application/json" } }
          )
        )
        .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 500 }));

      const config = useConfig((key: string) => key);
      config.bearerToken.value = "token";
      await config.connectGateway();
      config.form.default_model = "b";

      await config.saveSection("general");

      expect(config.errorMessage.value).toBe("form.saveError");
      expect(config.sectionSaving.general).toBe(false);
    });

    it("preserves webhook secret edits and reports success when save and reconnect both succeed", async () => {
      fetchMock
        .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
        .mockResolvedValueOnce(
          new Response(
            JSON.stringify({ config: { default_model: "a", channels: { webhook: {} } } }),
            { status: 200, headers: { "Content-Type": "application/json" } }
          )
        )
        .mockResolvedValueOnce(new Response(JSON.stringify({ updated: true }), { status: 200 }))
        .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
        .mockResolvedValueOnce(
          new Response(
            JSON.stringify({ config: { default_model: "b", channels: { webhook: {} } } }),
            { status: 200, headers: { "Content-Type": "application/json" } }
          )
        );

      const config = useConfig((key: string) => key);
      config.bearerToken.value = "token";
      await config.connectGateway();
      config.form.webhook_secret_mode = "replace";
      config.form.webhook_secret_value = "new-secret";
      config.form.default_model = "b";

      await config.saveSection("general");

      expect(config.form.webhook_secret_mode).toBe("replace");
      expect(config.form.webhook_secret_value).toBe("new-secret");
      expect(config.statusMessage.value).toBe("form.saveSuccess");
    });

    it("preserves webhook secret edits when reconnect fails after saving general settings", async () => {
      fetchMock
        .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
        .mockResolvedValueOnce(
          new Response(
            JSON.stringify({ config: { default_model: "a", channels: { webhook: {} } } }),
            { status: 200, headers: { "Content-Type": "application/json" } }
          )
        )
        .mockResolvedValueOnce(new Response(JSON.stringify({ updated: true }), { status: 200 }))
        .mockResolvedValueOnce(
          new Response(JSON.stringify({ error: "unauthorized" }), { status: 401 })
        );

      const config = useConfig((key: string) => key);
      config.bearerToken.value = "token";
      await config.connectGateway();
      config.form.webhook_secret_mode = "replace";
      config.form.webhook_secret_value = "new-secret";
      config.form.default_model = "b";

      await config.saveSection("general");

      expect(config.form.webhook_secret_mode).toBe("replace");
      expect(config.form.webhook_secret_value).toBe("new-secret");
      expect(config.statusMessage.value).toBe("");
      expect(config.errorMessage.value).toBe("auth.credentialInvalid");
      expect(config.bearerToken.value).toBe("");
    });
  });

  describe("connectGateway additional paths", () => {
    it("returns false with insecureUrlError when URL is insecure and bearer token is set", async () => {
      const config = useConfig((key: string) => key);
      config.baseUrl.value = "https://example.com/api";
      config.bearerToken.value = "secret-token";

      const result = await config.connectGateway();

      expect(result).toBe(false);
      expect(config.errorMessage.value).toBe("errors.insecureUrlError");
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("calls handleTransportFailure when options response is not ok and not 401/403", async () => {
      fetchMock.mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 500 }));

      const config = useConfig((key: string) => key);
      config.baseUrl.value = "http://localhost:3000/api";
      config.bearerToken.value = "token";

      const result = await config.connectGateway();

      expect(result).toBe(false);
      expect(config.errorMessage.value).toBe("auth.loadError");
      expect(config.onboardingState.value.state).toBe("blocked");
      expect(config.onboardingState.value.recoveryKind).toBe("paired_but_not_connected");
    });

    it("clears credentials when config response returns 401", async () => {
      fetchMock
        .mockResolvedValueOnce(
          new Response(JSON.stringify({}), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          })
        )
        .mockResolvedValueOnce(
          new Response(JSON.stringify({ error: "Unauthorized" }), { status: 401 })
        );

      const config = useConfig((key: string) => key);
      config.baseUrl.value = "http://localhost:3000/api";
      config.bearerToken.value = "stale-token";

      const result = await config.connectGateway();

      expect(result).toBe(false);
      expect(config.bearerToken.value).toBe("");
      expect(config.onboardingState.value.state).toBe("blocked");
      expect(config.onboardingState.value.recoveryKind).toBe("credential_invalid");
      expect(config.errorMessage.value).toBe("auth.credentialInvalid");
    });

    it("calls handleTransportFailure when config response is not ok and not 401/403", async () => {
      fetchMock
        .mockResolvedValueOnce(
          new Response(JSON.stringify({}), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          })
        )
        .mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 502 }));

      const config = useConfig((key: string) => key);
      config.baseUrl.value = "http://localhost:3000/api";
      config.bearerToken.value = "token";

      const result = await config.connectGateway();

      expect(result).toBe(false);
      expect(config.errorMessage.value).toBe("auth.loadError");
      expect(config.onboardingState.value.state).toBe("blocked");
      expect(config.onboardingState.value.recoveryKind).toBe("paired_but_not_connected");
    });

    it("calls handleTransportFailure when fetch throws a network error", async () => {
      fetchMock.mockRejectedValueOnce(new TypeError("Failed to fetch"));

      const config = useConfig((key: string) => key);
      config.baseUrl.value = "http://localhost:3000/api";
      config.bearerToken.value = "token";

      const result = await config.connectGateway();

      expect(result).toBe(false);
      expect(config.errorMessage.value).toBe("auth.loadError");
      expect(config.onboardingState.value.state).toBe("blocked");
      expect(config.onboardingState.value.recoveryKind).toBe("paired_but_not_connected");
    });

    it("resets quickPairState from failed to idle on entry", async () => {
      fetchMock.mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 500 }));

      const config = useConfig((key: string) => key);
      config.baseUrl.value = "http://localhost:3000/api";
      config.quickPairState.value = "failed";

      await config.connectGateway();

      expect(config.quickPairState.value).toBe("idle");
    });

    it("does not send Authorization header when no bearer token is present", async () => {
      fetchMock
        .mockResolvedValueOnce(
          new Response(JSON.stringify({}), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          })
        )
        .mockResolvedValueOnce(
          new Response(JSON.stringify({ config: { channels: { webhook: {} } } }), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          })
        );

      const config = useConfig((key: string) => key);
      config.baseUrl.value = "https://example.com/api";

      expect(config.bearerToken.value).toBe("");

      await config.connectGateway();

      const headers = fetchMock.mock.calls[0]?.[1]?.headers as Record<string, string>;
      expect(headers).toBeDefined();
      expect(headers.Authorization).toBeUndefined();
    });

    it("sends Authorization header and marks trust before transport when bearer token is present", async () => {
      fetchMock
        .mockResolvedValueOnce(
          new Response(JSON.stringify({}), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          })
        )
        .mockResolvedValueOnce(
          new Response(JSON.stringify({ config: { channels: { webhook: {} } } }), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          })
        );

      const config = useConfig((key: string) => key);
      config.baseUrl.value = "http://localhost:3000/api";
      config.bearerToken.value = "token";

      await config.connectGateway();

      const headers = fetchMock.mock.calls[0]?.[1]?.headers as Record<string, string>;
      expect(headers.Authorization).toBe("Bearer token");
      expect(config.onboardingState.value.state).toBe("ready");
    });
  });
});
