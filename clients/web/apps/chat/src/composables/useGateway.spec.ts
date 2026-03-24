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
});
