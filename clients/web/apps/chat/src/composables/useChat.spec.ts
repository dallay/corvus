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
    const webhookCall = fetchMock.mock.calls.find(([url]) => String(url).includes("/webhook"));
    expect(webhookCall).toBeDefined();
    const init = webhookCall?.[1];
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

    const webhookCall = fetchMock.mock.calls.find(([url]) => String(url).includes("/webhook"));
    expect(webhookCall).toBeDefined();
    const init = webhookCall?.[1];
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
        controller.enqueue(encoder.encode('lo\r\n\r\nevent: done\r\ndata: {"session_id":"'));
        controller.enqueue(
          encoder.encode('90909090-9090-4090-8090-909090909090","message_id":"msg-001"}\r\n\r\n')
        );
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

  it("streamMessage surfaces recalled_memory_keys from done event", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("90909090-9090-4090-8090-909090909090");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    const encoder = new TextEncoder();
    const donePayload = JSON.stringify({
      session_id: "90909090-9090-4090-8090-909090909090",
      message_id: "msg-002",
      recalled_memory_keys: ["memory_recall"],
    });
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(encoder.encode("event: chunk\r\ndata: Sure!\r\n\r\n"));
        controller.enqueue(encoder.encode(`event: done\r\ndata: ${donePayload}\r\n\r\n`));
        controller.close();
      },
    });

    fetchMock.mockResolvedValueOnce(
      new Response(stream, {
        status: 200,
        headers: { "Content-Type": "text/event-stream" },
      })
    );

    const doneEvent = await chat.streamMessage("hi", () => undefined);
    expect(doneEvent.recalled_memory_keys).toEqual(["memory_recall"]);
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

  it("sendMessage returns approval_required on 403 with approval_required code", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("88888888-8888-4888-8888-888888888888");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    fetchMock.mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          error: { code: "approval_required", tool: "shell_exec", reason: "dangerous" },
          session_id: "88888888-8888-4888-8888-888888888888",
        }),
        { status: 403, headers: { "Content-Type": "application/json" } }
      )
    );

    const result = await chat.sendMessage("run rm -rf");

    expect(result).toEqual({
      type: "approval_required",
      tool: "shell_exec",
      reason: "dangerous",
      sessionId: "88888888-8888-4888-8888-888888888888",
    });
    expect(chat.currentSessionId.value).toBe("88888888-8888-4888-8888-888888888888");
    expect(gateway.onboardingState.value.state).toBe("ready");
  });

  it("streamMessage throws streamUnavailable on non-ok non-auth response", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("a1a1a1a1-a1a1-4a1a-8a1a-a1a1a1a1a1a1");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    fetchMock.mockResolvedValueOnce(new Response(null, { status: 500 }));

    await expect(chat.streamMessage("hello", () => undefined)).rejects.toThrow(
      "chat.streamUnavailable"
    );
  });

  it("streamMessage throws connectBeforeChat when gateway is not ready", async () => {
    const gateway = useGateway((key: string) => key);
    const chat = useChat((key: string) => key, gateway);

    await expect(chat.streamMessage("hello", () => undefined)).rejects.toThrow(
      "chat.connectBeforeChat"
    );
  });

  it("streamMessage throws emptyMessageError for blank message", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("b2b2b2b2-b2b2-4b2b-8b2b-b2b2b2b2b2b2");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    await expect(chat.streamMessage("   ", () => undefined)).rejects.toThrow(
      "chat.emptyMessageError"
    );
  });

  it("streamMessage throws timeoutError on abort", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("c3c3c3c3-c3c3-4c3c-8c3c-c3c3c3c3c3c3");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    const abortError = new DOMException("The operation was aborted.", "AbortError");
    fetchMock.mockRejectedValueOnce(abortError);

    await expect(chat.streamMessage("hello", () => undefined)).rejects.toThrow("chat.timeoutError");
  });

  it("streamMessage handles 401 without approval code as credential invalid", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("d4d4d4d4-d4d4-4d4d-8d4d-d4d4d4d4d4d4");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ error: "unauthorized" }), {
        status: 401,
        headers: { "Content-Type": "application/json" },
      })
    );

    await expect(chat.streamMessage("hello", () => undefined)).rejects.toThrow(
      "auth.credentialInvalid"
    );
    expect(chat.currentSessionId.value).toBe("");
  });

  it("streamMessage throws when body reader is null", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("e5e5e5e5-e5e5-4e5e-8e5e-e5e5e5e5e5e5");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    const response = new Response(null, {
      status: 200,
      headers: { "Content-Type": "text/event-stream" },
    });
    Object.defineProperty(response, "body", { value: null });
    fetchMock.mockResolvedValueOnce(response);

    await expect(chat.streamMessage("hello", () => undefined)).rejects.toThrow(
      "chat.streamUnavailable"
    );
  });

  it("streamMessage throws on SSE error event", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("f6f6f6f6-f6f6-4f6f-8f6f-f6f6f6f6f6f6");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    const encoder = new TextEncoder();
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(
          encoder.encode(
            'event: error\r\ndata: {"message":"provider_error","code":"provider_unavailable"}\r\n\r\n'
          )
        );
        controller.close();
      },
    });

    fetchMock.mockResolvedValueOnce(
      new Response(stream, {
        status: 200,
        headers: { "Content-Type": "text/event-stream" },
      })
    );

    await expect(chat.streamMessage("hello", () => undefined)).rejects.toThrow("provider_error");
  });

  it("streamMessage throws when stream ends without done event", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValue("17171717-1717-4717-8717-171717171717");
    const gateway = await connectReadyGateway();
    const chat = useChat((key: string) => key, gateway);
    chat.createSession();

    const encoder = new TextEncoder();
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(encoder.encode("event: chunk\r\ndata: partial\r\n\r\n"));
        controller.close();
      },
    });

    fetchMock.mockResolvedValueOnce(
      new Response(stream, {
        status: 200,
        headers: { "Content-Type": "text/event-stream" },
      })
    );

    await expect(chat.streamMessage("hello", () => undefined)).rejects.toThrow("chat.requestError");
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

  describe("session list and switching", () => {
    it("fetchSessionList calls gateway and populates sessionList ref", async () => {
      vi.spyOn(crypto, "randomUUID").mockReturnValue("aaa11111-1111-4111-8111-111111111111");
      const gateway = await connectReadyGateway();
      const chat = useChat((key: string) => key, gateway);

      const sessionListPayload = {
        sessions: [
          {
            id: "s1",
            started_at: "2026-01-01",
            ended_at: null,
            message_count: 5,
            last_activity: "2026-01-02",
          },
          {
            id: "s2",
            started_at: "2026-01-02",
            ended_at: "2026-01-03",
            message_count: 3,
            last_activity: "2026-01-03",
          },
        ],
        total: 2,
      };

      // createSession triggers one watcher-driven fetch, and the explicit call triggers another.
      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify(sessionListPayload), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );
      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify(sessionListPayload), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      chat.createSession();
      await chat.fetchSessionList();

      expect(chat.sessionList.value).toHaveLength(2);
      expect(chat.sessionList.value[0]?.id).toBe("s1");
      expect(chat.sessionList.value[1]?.id).toBe("s2");
    });

    it("clears stale sessionList when gateway auth context changes", async () => {
      vi.spyOn(crypto, "randomUUID").mockReturnValue("fff66666-6666-4666-8666-666666666666");
      const gateway = await connectReadyGateway();
      const chat = useChat((key: string) => key, gateway);

      fetchMock.mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            sessions: [
              {
                id: "s1",
                started_at: "2026-01-01",
                ended_at: null,
                message_count: 5,
                last_activity: "2026-01-02",
              },
            ],
            total: 1,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        )
      );

      chat.createSession();
      await vi.waitFor(() => expect(chat.sessionList.value).toHaveLength(1));

      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify({ sessions: [], total: 0 }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      gateway.bearerToken.value = "token-456";

      await vi.waitFor(() => expect(chat.sessionList.value).toEqual([]));
      await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    });

    it("fetchSessionList handles 404 gracefully with empty list", async () => {
      vi.spyOn(crypto, "randomUUID").mockReturnValue("bbb22222-2222-4222-8222-222222222222");
      const gateway = await connectReadyGateway();
      const chat = useChat((key: string) => key, gateway);
      chat.createSession();

      fetchMock.mockResolvedValueOnce(new Response(null, { status: 404 }));

      await chat.fetchSessionList();

      expect(chat.sessionList.value).toEqual([]);
    });

    it("switchSession sets new session ID and persists it", async () => {
      vi.spyOn(crypto, "randomUUID").mockReturnValue("ccc33333-3333-4333-8333-333333333333");
      const gateway = await connectReadyGateway();
      const chat = useChat((key: string) => key, gateway);
      chat.createSession();

      expect(chat.currentSessionId.value).toBe("ccc33333-3333-4333-8333-333333333333");

      chat.switchSession("target-session-id");

      expect(chat.currentSessionId.value).toBe("target-session-id");
      expect(chat.sessionState.value.state).toBe("session_ready");
      expect(window.sessionStorage.getItem("corvus.chat.session:%2Fapi")).toBe("target-session-id");
    });

    it("switchSession ignores switch to same session", async () => {
      vi.spyOn(crypto, "randomUUID").mockReturnValue("ddd44444-4444-4444-8444-444444444444");
      const gateway = await connectReadyGateway();
      const chat = useChat((key: string) => key, gateway);
      chat.createSession();

      chat.switchSession("ddd44444-4444-4444-8444-444444444444");

      expect(chat.currentSessionId.value).toBe("ddd44444-4444-4444-8444-444444444444");
    });

    it("switchSession ignores empty target", async () => {
      vi.spyOn(crypto, "randomUUID").mockReturnValue("eee55555-5555-4555-8555-555555555555");
      const gateway = await connectReadyGateway();
      const chat = useChat((key: string) => key, gateway);
      chat.createSession();

      chat.switchSession("");

      expect(chat.currentSessionId.value).toBe("eee55555-5555-4555-8555-555555555555");
    });

    it("fetchSessionList does nothing when gateway is not ready", async () => {
      const gateway = useGateway((key: string) => key);
      const chat = useChat((key: string) => key, gateway);

      await chat.fetchSessionList();

      expect(chat.sessionList.value).toEqual([]);
      // Only the connectReadyGateway call from other tests uses fetch; here no fetch calls
      expect(fetchMock).not.toHaveBeenCalled();
    });
  });
});
