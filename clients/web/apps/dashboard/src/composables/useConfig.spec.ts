import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useConfig } from "@/composables/useConfig";

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
      expect(config.errorMessage.value).toBe("auth.loadError");
      expect(config.pairingCode.value).toBe("");
      expect(config.bearerToken.value).toBe("");
    });
  });
});
