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

    expect(reply).toEqual({ type: "message", content: "ok" });
    expect(fetchMock).toHaveBeenCalledTimes(2);
    const [, init] = fetchMock.mock.calls[1] ?? [];
    expect((init?.headers as Record<string, string>).Authorization).toBe("Bearer token-123");
    expect((init?.headers as Record<string, string>)["X-Session-Id"]).toBe(
      "22222222-2222-4222-8222-222222222222"
    );
  });

  it("sends the provided request id through the webhook path", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("12121212-1212-4212-8212-121212121212");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ response: "ok", session_id: chat.currentSessionId.value }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );

    await chat.sendMessage("hola", "shared-request-id");

    const [, init] = fetchMock.mock.calls[1] ?? [];
    expect((init?.headers as Record<string, string>)["X-Idempotency-Key"]).toBe(
      "shared-request-id"
    );
    expect(init?.body).toBe(JSON.stringify({ message: "hola", request_id: "shared-request-id" }));
  });

  it("sendMessage throws connectBeforeChat when gateway is not ready", async () => {
    const gateway = useGateway((key: string) => key);
    const chat = useChat((key: string) => key, gateway);

    await expect(chat.sendMessage("hello")).rejects.toThrow("chat.connectBeforeChat");
  });

  it("sendMessage throws emptyMessageError for empty or whitespace message", async () => {
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    await expect(chat.sendMessage("")).rejects.toThrow("chat.emptyMessageError");
    await expect(chat.sendMessage("   ")).rejects.toThrow("chat.emptyMessageError");
  });

  it("sendMessage calls markCredentialInvalid and clearSession on 401", async () => {
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    fetchMock.mockResolvedValueOnce(new Response(null, { status: 401 }));

    await expect(chat.sendMessage("hello")).rejects.toThrow("auth.credentialInvalid");
    expect(chat.currentSessionId.value).toBe("");
    expect(gateway.onboardingState.value.state).not.toBe("ready");
  });

  it("sendMessage calls markCredentialInvalid and clearSession on 403", async () => {
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    fetchMock.mockResolvedValueOnce(new Response(null, { status: 403 }));

    await expect(chat.sendMessage("hello")).rejects.toThrow("auth.credentialInvalid");
    expect(chat.currentSessionId.value).toBe("");
  });

  it("sendMessage calls markPairedButNotConnected on non-ok response", async () => {
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    fetchMock.mockResolvedValueOnce(new Response(null, { status: 500 }));

    await expect(chat.sendMessage("hello")).rejects.toThrow("chat.requestError");
  });

  it("sendMessage throws timeoutError when fetch aborts", async () => {
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    const abortError = new DOMException("The operation was aborted.", "AbortError");
    fetchMock.mockRejectedValueOnce(abortError);

    await expect(chat.sendMessage("hello")).rejects.toThrow("chat.timeoutError");
  });

  it("sendMessage re-throws generic errors", async () => {
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    fetchMock.mockRejectedValueOnce(new Error("network failure"));

    await expect(chat.sendMessage("hello")).rejects.toThrow("network failure");
  });

  it("streamMessage preserves SSE frames split across transport chunks", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("90909090-9090-4090-8090-909090909090");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    const encoder = new TextEncoder();
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(encoder.encode("event: chunk\r\ndata: Hel"));
        controller.enqueue(encoder.encode("lo\r\n\r\nevent: done\r\ndata: {\"session_id\":\""));
        controller.enqueue(encoder.encode("90909090-9090-4090-8090-909090909090\"}\r\n\r\n"));
        controller.close();
      },
    });

    fetchMock.mockResolvedValueOnce(
      new Response(stream, {
        status: 200,
        headers: { "Content-Type": "text/event-stream" },
      })
    );

    const chunks: string[] = [];
    const doneEvent = await chat.streamMessage("hello", (chunk) => chunks.push(chunk), "req-1");

    expect(chunks).toEqual(["Hello"]);
    expect(doneEvent.session_id).toBe("90909090-9090-4090-8090-909090909090");
    const [, init] = fetchMock.mock.calls[1] ?? [];
    expect((init?.headers as Record<string, string>)["X-Request-Id"]).toBe("req-1");
    expect(init?.body).toBe(JSON.stringify({ message: "hello", request_id: "req-1" }));
  });

  it("streamMessage keeps approval responses eligible for webhook fallback", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("34343434-3434-4434-8434-343434343434");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ error: { code: "approval_required" } }), {
        status: 403,
        headers: { "Content-Type": "application/json" },
      })
    );

    await expect(chat.streamMessage("hello", () => undefined)).rejects.toThrow(
      "chat.streamUnavailable"
    );
    expect(chat.currentSessionId.value).toBe("34343434-3434-4434-8434-343434343434");
    expect(gateway.onboardingState.value.state).toBe("ready");
  });

  it("clearSession clears sessionId, removes from storage, and sets correct state", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("44444444-4444-4444-8444-444444444444");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    expect(chat.currentSessionId.value).toBe("44444444-4444-4444-8444-444444444444");

    chat.clearSession();

    expect(chat.currentSessionId.value).toBe("");
    expect(window.sessionStorage.getItem("corvus.chat.session:%2Fapi")).toBeNull();
    expect(chat.sessionState.value.state).toBe("session_pending");
    expect(chat.statusMessage.value).toBe("chat.sessionCleared");
  });

  it("clearSession sets idle when gateway is not ready", () => {
    const gateway = useGateway((key: string) => key);
    const chat = useChat((key: string) => key, gateway);

    chat.clearSession();

    expect(chat.sessionState.value.state).toBe("idle");
  });

  it("startSession(false) always creates a new session even if stored session exists", async () => {
    window.sessionStorage.setItem("corvus.chat.session:%2Fapi", "stored-session-id");
    vi.spyOn(crypto, "randomUUID").mockReturnValue("55555555-5555-4555-8555-555555555555");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);

    const started = chat.startSession(false);

    expect(started).toBe(true);
    expect(chat.currentSessionId.value).toBe("55555555-5555-4555-8555-555555555555");
    expect(chat.currentSessionId.value).not.toBe("stored-session-id");
  });

  it("resets session when gateway baseUrl changes", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("66666666-6666-4666-8666-666666666666");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    expect(chat.currentSessionId.value).toBe("66666666-6666-4666-8666-666666666666");

    gateway.baseUrl.value = "https://other-host:9999";

    await vi.dynamicImportSettled();

    expect(chat.currentSessionId.value).toBe("");
  });

  it("sendMessage does not overwrite session_id when session is already ready", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("77777777-7777-4777-8777-777777777777");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    expect(chat.currentSessionId.value).toBe("77777777-7777-4777-8777-777777777777");

    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ response: "hi", session_id: "server-overwrite-attempt" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );

    const reply = await chat.sendMessage("hello");

    expect(reply).toEqual({ type: "message", content: "hi" });
    expect(chat.currentSessionId.value).toBe("77777777-7777-4777-8777-777777777777");
  });
});
