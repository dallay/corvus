import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  useGateway,
  webChatOnboardingRecoveryLabel,
  webChatOnboardingTransitionLabel,
} from "@/composables/useGateway";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

describe("useGateway", () => {
  it("maps /health -> /pair -> ready using the canonical HTTP onboarding states", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: false }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ paired: true, token: "token-123" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

    const gateway = useGateway((key: string) => key);
    gateway.pairingCode.value = "857258";

    const paired = await gateway.pairGateway();

    expect(paired).toBe(true);
    expect(gateway.onboardingState.value.state).toBe("ready");
    expect(gateway.bearerToken.value).toBe("token-123");
    expect(gateway.pairingCode.value).toBe("");
    expect(fetchMock.mock.calls[0]?.[0]).toBe("http://localhost:3000/api/health");
    expect(fetchMock.mock.calls[1]?.[0]).toBe("http://localhost:3000/api/pair");
    expect(fetchMock.mock.calls[2]?.[0]).toBe("http://localhost:3000/api/health");
  });

  it("allows HTTPS to non-localhost hosts for remote deployments", () => {
    const gateway = useGateway((key: string) => key);
    expect(gateway.isUrlSafeForSecrets("https://corvus.example.com/api")).toBe(true);
    expect(gateway.isUrlSafeForSecrets("https://10.0.1.5:8443/api")).toBe(true);
  });

  it("rejects HTTP to non-localhost hosts", () => {
    const gateway = useGateway((key: string) => key);
    expect(gateway.isUrlSafeForSecrets("http://corvus.example.com/api")).toBe(false);
    expect(gateway.isUrlSafeForSecrets("http://10.0.1.5:8080/api")).toBe(false);
  });

  it("allows HTTP to localhost variants", () => {
    const gateway = useGateway((key: string) => key);
    expect(gateway.isUrlSafeForSecrets("http://localhost:3000/api")).toBe(true);
    expect(gateway.isUrlSafeForSecrets("http://127.0.0.1:3000/api")).toBe(true);
  });

  it("maps invalid pairing input to the normalized trust recovery state", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: false }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "Invalid pairing code" }), {
          status: 403,
          headers: { "Content-Type": "application/json" },
        })
      );

    const gateway = useGateway((key: string) => key);
    gateway.pairingCode.value = "wrong";

    const paired = await gateway.pairGateway({ autoConnect: false });

    expect(paired).toBe(false);
    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.onboardingState.value.recoveryKind).toBe("trust_input_invalid");
    expect(gateway.lastTransitionLabel.value).toBe("runtime_path_confirmed__to__blocked");
    expect(gateway.currentRecoveryLabel.value).toBe("trust_input_invalid");
    expect(gateway.errorMessage.value).toBe("auth.pairingInvalid");
    expect(gateway.pairingCode.value).toBe("wrong");
  });

  it("exposes canonical observability labels for onboarding transitions and recovery", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: false }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ paired: true, token: "token-123" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

    const gateway = useGateway((key: string) => key);
    gateway.pairingCode.value = "857258";

    const paired = await gateway.pairGateway({ autoConnect: false });

    expect(paired).toBe(true);
    expect(gateway.lastTransitionLabel.value).toBe("runtime_path_confirmed__to__trust_established");
    expect(gateway.currentRecoveryLabel.value).toBe(null);
    expect(webChatOnboardingTransitionLabel("trust_pending", "trust_established")).toBe(
      "trust_pending__to__trust_established"
    );
    expect(webChatOnboardingRecoveryLabel("paired_but_not_connected")).toBe(
      "paired_but_not_connected"
    );
  });

  it("maps expired pairing input to the normalized trust recovery state", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: false }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "Pairing code expired" }), {
          status: 410,
          headers: { "Content-Type": "application/json" },
        })
      );

    const gateway = useGateway((key: string) => key);
    gateway.pairingCode.value = "stale";

    const paired = await gateway.pairGateway({ autoConnect: false });

    expect(paired).toBe(false);
    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.onboardingState.value.recoveryKind).toBe("trust_input_expired");
    expect(gateway.errorMessage.value).toBe("auth.pairingExpired");
  });

  it("maps missing bearer tokens to credential_missing when runtime is paired already", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ status: "ok", paired: true }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );

    const gateway = useGateway((key: string) => key);

    const connected = await gateway.connectGateway();

    expect(connected).toBe(false);
    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.onboardingState.value.recoveryKind).toBe("credential_missing");
    expect(gateway.errorMessage.value).toBe("auth.credentialMissing");
  });

  it("clears stale bearer tokens when the chat gateway marks credentials invalid", () => {
    const gateway = useGateway((key: string) => key);
    gateway.bearerToken.value = "stale-token";

    gateway.markCredentialInvalid();

    expect(gateway.bearerToken.value).toBe("");
    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.onboardingState.value.recoveryKind).toBe("credential_invalid");
    expect(gateway.errorMessage.value).toBe("auth.credentialInvalid");
  });

  it("preserves trust progress when the surface is paired but cannot reconnect", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: false }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ paired: true, token: "token-123" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockRejectedValueOnce(new Error("offline"));

    const gateway = useGateway((key: string) => key);
    gateway.pairingCode.value = "857258";

    const paired = await gateway.pairGateway();

    expect(paired).toBe(false);
    expect(gateway.bearerToken.value).toBe("token-123");
    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.onboardingState.value.recoveryKind).toBe("paired_but_not_connected");
    expect(gateway.onboardingState.value.canResume).toBe(true);
    expect(gateway.onboardingSteps.value[1]?.status).toBe("complete");
    expect(gateway.onboardingSteps.value[2]?.status).toBe("blocked");
  });

  it("connectGateway returns false and sets insecureUrlError when URL is insecure with credentials", async () => {
    const gateway = useGateway((key: string) => key);
    gateway.baseUrl.value = "http://remote.example.com/api";
    gateway.bearerToken.value = "some-token";

    const result = await gateway.connectGateway();

    expect(result).toBe(false);
    expect(gateway.errorMessage.value).toBe("errors.insecureUrlError");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("connectGateway returns false and sets insecureUrlError when webhook secret on insecure URL", async () => {
    const gateway = useGateway((key: string) => key);
    gateway.baseUrl.value = "http://remote.example.com/api";
    gateway.webhookSecret.value = "secret-value";

    const result = await gateway.connectGateway();

    expect(result).toBe(false);
    expect(gateway.errorMessage.value).toBe("errors.insecureUrlError");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("connectGateway clears credentials when health returns paired:false but bearer token exists", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ status: "ok", paired: false }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );

    const gateway = useGateway((key: string) => key);
    gateway.bearerToken.value = "stale-bearer";

    const result = await gateway.connectGateway();

    expect(result).toBe(false);
    expect(gateway.bearerToken.value).toBe("");
    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.onboardingState.value.recoveryKind).toBe("credential_invalid");
    expect(gateway.errorMessage.value).toBe("auth.credentialInvalid");
  });

  it("connectGateway transitions to trust_pending when health returns paired:false and no bearer token", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ status: "ok", paired: false }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );

    const gateway = useGateway((key: string) => key);

    const result = await gateway.connectGateway();

    expect(result).toBe(false);
    expect(gateway.onboardingState.value.state).toBe("trust_pending");
    expect(gateway.onboardingState.value.recoveryKind).toBeNull();
  });

  it("connectGateway handles fetch throw via handleTransportFailure", async () => {
    fetchMock.mockRejectedValueOnce(new Error("network failure"));

    const gateway = useGateway((key: string) => key);

    const result = await gateway.connectGateway();

    expect(result).toBe(false);
    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.errorMessage.value).toBe("chatOnboarding.loadError");
    expect(gateway.loading.value).toBe(false);
  });

  it("pairGateway returns false when pairing code is empty", async () => {
    const gateway = useGateway((key: string) => key);
    gateway.pairingCode.value = "   ";

    const result = await gateway.pairGateway();

    expect(result).toBe(false);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("pairGateway returns false and sets insecureUrlError for insecure URL", async () => {
    const gateway = useGateway((key: string) => key);
    gateway.baseUrl.value = "http://remote.example.com/api";
    gateway.pairingCode.value = "123456";

    const result = await gateway.pairGateway();

    expect(result).toBe(false);
    expect(gateway.errorMessage.value).toBe("errors.insecureUrlError");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("pairGateway blocks with runtime_unavailable when health check fails", async () => {
    fetchMock.mockResolvedValueOnce(new Response("", { status: 503 }));

    const gateway = useGateway((key: string) => key);
    gateway.pairingCode.value = "123456";

    const result = await gateway.pairGateway({ autoConnect: false });

    expect(result).toBe(false);
    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.onboardingState.value.recoveryKind).toBe("runtime_unavailable");
    expect(gateway.errorMessage.value).toBe("chatOnboarding.loadError");
  });

  it("pairGateway blocks with transport_unavailable when pair response is not ok and not expired/401/403", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: false }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "internal error" }), {
          status: 500,
          headers: { "Content-Type": "application/json" },
        })
      );

    const gateway = useGateway((key: string) => key);
    gateway.pairingCode.value = "123456";

    const result = await gateway.pairGateway({ autoConnect: false });

    expect(result).toBe(false);
    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.onboardingState.value.recoveryKind).toBe("transport_unavailable");
    expect(gateway.errorMessage.value).toBe("chatOnboarding.loadError");
  });

  it("pairGateway blocks with transport_unavailable when pair returns ok but no token", async () => {
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: false }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ paired: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

    const gateway = useGateway((key: string) => key);
    gateway.pairingCode.value = "123456";

    const result = await gateway.pairGateway({ autoConnect: false });

    expect(result).toBe(false);
    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.onboardingState.value.recoveryKind).toBe("transport_unavailable");
    expect(gateway.errorMessage.value).toBe("form.pairingMissingTokenError");
  });

  it("pairGateway catch block sets runtime_unavailable when fetch throws", async () => {
    fetchMock.mockRejectedValueOnce(new Error("network down"));

    const gateway = useGateway((key: string) => key);
    gateway.pairingCode.value = "123456";

    const result = await gateway.pairGateway({ autoConnect: false });

    expect(result).toBe(false);
    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.onboardingState.value.recoveryKind).toBe("runtime_unavailable");
    expect(gateway.errorMessage.value).toBe("chatOnboarding.loadError");
    expect(gateway.loading.value).toBe(false);
  });

  it("markPairedButNotConnected sets blocked paired_but_not_connected state", () => {
    const gateway = useGateway((key: string) => key);

    gateway.markPairedButNotConnected();

    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.onboardingState.value.recoveryKind).toBe("paired_but_not_connected");
    expect(gateway.onboardingState.value.canRetry).toBe(true);
    expect(gateway.onboardingState.value.canResume).toBe(true);
    expect(gateway.errorMessage.value).toBe("chatOnboarding.transportDisconnected");
  });

  it("resetMessages clears error and status messages", () => {
    const gateway = useGateway((key: string) => key);
    gateway.errorMessage.value = "some error";
    gateway.statusMessage.value = "some status";

    gateway.resetMessages();

    expect(gateway.errorMessage.value).toBe("");
    expect(gateway.statusMessage.value).toBe("");
  });

  it("handleTransportFailure without prior trust sets runtime_unavailable via health failure", async () => {
    // When connectGateway gets a null health (not an exception), handleTransportFailure
    // is called. Since markTransportConnecting() sets trustEstablished=true before the
    // health call, a health-null path still sees trustEstablished. To exercise the
    // runtime_unavailable branch of handleTransportFailure we need a fresh gateway
    // where trustEstablished is false and bearerToken is empty — which happens when
    // pairGateway's catch fires before markRuntimeConfirmed.
    fetchMock.mockRejectedValueOnce(new Error("offline"));

    const gateway = useGateway((key: string) => key);
    gateway.pairingCode.value = "123456";

    await gateway.pairGateway({ autoConnect: false });

    expect(gateway.onboardingState.value.state).toBe("blocked");
    expect(gateway.onboardingState.value.recoveryKind).toBe("runtime_unavailable");
    expect(gateway.onboardingState.value.canResume).toBe(false);
  });

  it("isUrlSafeForSecrets returns false for unparseable URLs", () => {
    const gateway = useGateway((key: string) => key);
    expect(gateway.isUrlSafeForSecrets("://not-a-url")).toBe(false);
  });

  it("isUrlSafeForSecrets returns false for URLs with username or password", () => {
    const gateway = useGateway((key: string) => key);
    expect(gateway.isUrlSafeForSecrets("https://user:pass@example.com/api")).toBe(false);
    expect(gateway.isUrlSafeForSecrets("https://user@example.com/api")).toBe(false);
  });

  it("isUrlSafeForSecrets returns false for URLs with search params or hash", () => {
    const gateway = useGateway((key: string) => key);
    expect(gateway.isUrlSafeForSecrets("https://example.com/api?key=val")).toBe(false);
    expect(gateway.isUrlSafeForSecrets("https://example.com/api#fragment")).toBe(false);
  });

  it("gatewayUrl with non-relative base URL joins path correctly", () => {
    const gateway = useGateway((key: string) => key);
    gateway.baseUrl.value = "https://remote.example.com/api";

    const url = gateway.gatewayUrl("/health");
    expect(url).toBe("https://remote.example.com/api/health");
  });

  it("authHeaders includes webhook secret when set", () => {
    const gateway = useGateway((key: string) => key);
    gateway.bearerToken.value = "my-token";
    gateway.webhookSecret.value = "my-secret";

    const headers = gateway.authHeaders();

    expect(headers["Content-Type"]).toBe("application/json");
    expect(headers.Authorization).toBe("Bearer my-token");
    expect(headers["X-Webhook-Secret"]).toBe("my-secret");
  });

  it("authHeaders omits Content-Type when includeJsonContentType is false", () => {
    const gateway = useGateway((key: string) => key);
    gateway.bearerToken.value = "my-token";

    const headers = gateway.authHeaders(false);

    expect(headers["Content-Type"]).toBeUndefined();
    expect(headers.Authorization).toBe("Bearer my-token");
  });

  it("createIdempotencyKey falls back to timestamp-based key when crypto.randomUUID is unavailable", () => {
    const originalCrypto = globalThis.crypto;
    vi.stubGlobal("crypto", { randomUUID: undefined });

    const gateway = useGateway((key: string) => key);
    const key = gateway.createIdempotencyKey();

    expect(key).toMatch(/^chat-\d+-[a-z0-9]+$/);

    vi.stubGlobal("crypto", originalCrypto);
  });
});
