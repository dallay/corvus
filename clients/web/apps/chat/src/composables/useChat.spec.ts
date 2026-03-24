import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  chatSessionRecoveryLabel,
  chatSessionTransitionLabel,
  useChat,
} from "@/composables/useChat";
import { useGateway } from "@/composables/useGateway";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
  window.sessionStorage.clear();
});

async function connectReadyGateway() {
  fetchMock.mockResolvedValueOnce(
    new Response(JSON.stringify({ status: "ok", paired: true }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    })
  );

  const gateway = useGateway((key: string) => key);
  gateway.bearerToken.value = "token-123";
  await gateway.connectGateway();
  return gateway;
}

describe("useChat", () => {
  it("keeps session lifecycle separate from onboarding readiness", async () => {
    const gateway = useGateway((key: string) => key);
    gateway.bearerToken.value = "token-123";

    const chat = useChat((key: string) => key, gateway);

    expect(gateway.onboardingState.value.state).toBe("trust_pending");
    expect(chat.sessionState.value.state).toBe("idle");
    expect(chat.createSession()).toBe(false);
  });

  it("enters session_pending only after gateway readiness and creates a UUID session", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("11111111-1111-4111-8111-111111111111");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);

    const started = chat.createSession();

    expect(started).toBe(true);
    expect(chat.sessionState.value.state).toBe("session_ready");
    expect(chat.currentSessionId.value).toBe("11111111-1111-4111-8111-111111111111");
    expect(window.sessionStorage.getItem("corvus.chat.session:%2Fapi")).toBe(
      "11111111-1111-4111-8111-111111111111"
    );
  });

  it("resumes a stored session when available", async () => {
    window.sessionStorage.setItem("corvus.chat.session:%2Fapi", "resume-session-1");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);

    const resumed = chat.resumeSession();

    expect(resumed).toBe(true);
    expect(chat.sessionState.value.state).toBe("session_ready");
    expect(chat.currentSessionId.value).toBe("resume-session-1");
    expect(chat.statusMessage.value).toBe("chat.sessionResumed");
  });

  it("maps missing resumable state to session_unavailable", async () => {
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);

    const resumed = chat.resumeSession();

    expect(resumed).toBe(false);
    expect(chat.sessionState.value.state).toBe("blocked");
    expect(chat.sessionState.value.recoveryKind).toBe("session_unavailable");
    expect(chat.lastTransitionLabel.value).toBe("session_pending__to__blocked");
    expect(chat.currentRecoveryLabel.value).toBe("session_unavailable");
    expect(chat.errorMessage.value).toBe("chat.sessionUnavailable");
  });

  it("tracks canonical session transition labels after gateway readiness", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("33333333-3333-4333-8333-333333333333");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);

    const started = chat.createSession();

    expect(started).toBe(true);
    expect(chat.lastTransitionLabel.value).toBe("session_pending__to__session_ready");
    expect(chat.currentRecoveryLabel.value).toBe(null);
    expect(chatSessionTransitionLabel("session_pending", "session_ready")).toBe(
      "session_pending__to__session_ready"
    );
    expect(chatSessionRecoveryLabel("session_unavailable")).toBe("session_unavailable");
  });

  it("sends chat turns with bearer and X-Session-Id after readiness", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("22222222-2222-4222-8222-222222222222");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    fetchMock.mockResolvedValueOnce(
      new Response(
        JSON.stringify({ response: "ok", session_id: "22222222-2222-4222-8222-222222222222" }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }
      )
    );

    const reply = await chat.sendMessage("hola");

    expect(reply).toBe("ok");
    expect(fetchMock).toHaveBeenCalledTimes(2);
    const [, init] = fetchMock.mock.calls[1] ?? [];
    expect((init?.headers as Record<string, string>).Authorization).toBe("Bearer token-123");
    expect((init?.headers as Record<string, string>)["X-Session-Id"]).toBe(
      "22222222-2222-4222-8222-222222222222"
    );
  });
});
