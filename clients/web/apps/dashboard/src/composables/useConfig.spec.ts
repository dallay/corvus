import { beforeEach, describe, expect, it, vi } from "vitest";

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

    const config = useConfig((key) => key);
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

    const config = useConfig((key) => key);
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

    const config = useConfig((key) => key);
    config.bearerToken.value = "token";
    await config.connectGateway();
    config.form.webhook_secret_mode = "replace";
    config.form.webhook_secret_value = "  ";

    await config.saveSection("webhook");

    expect(config.errorMessage.value).toBe("auth.emptyWebhookSecret");
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("uses same-origin proxied api endpoints by default", async () => {
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

    const config = useConfig((key) => key);
    config.pairingCode.value = "857258";

    await config.pairGateway();
    await config.connectGateway();

    expect(fetchMock.mock.calls[0]?.[0]).toBe("http://localhost:3000/api/pair");
    expect(fetchMock.mock.calls[1]?.[0]).toBe("http://localhost:3000/api/web/admin/options");
    expect(fetchMock.mock.calls[2]?.[0]).toBe("http://localhost:3000/api/web/admin/config");
  });

  it("preserves absolute base paths when building gateway urls", async () => {
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

    const config = useConfig((key) => key);
    config.baseUrl.value = "http://corvus.localhost/api";
    config.pairingCode.value = "857258";

    await config.pairGateway();
    await config.connectGateway();

    expect(fetchMock.mock.calls[0]?.[0]).toBe("http://corvus.localhost/api/pair");
    expect(fetchMock.mock.calls[1]?.[0]).toBe("http://corvus.localhost/api/web/admin/options");
    expect(fetchMock.mock.calls[2]?.[0]).toBe("http://corvus.localhost/api/web/admin/config");
  });

  it("blocks sending secrets to non-local origins", async () => {
    const config = useConfig((key) => key);
    config.baseUrl.value = "https://example.com/api";
    config.pairingCode.value = "857258";

    await config.pairGateway();

    expect(config.errorMessage.value).toBe("errors.insecureUrlError");
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
