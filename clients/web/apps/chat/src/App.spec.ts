import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createI18n } from "vue-i18n";

import App from "@/App.vue";
import { i18nConfig } from "@/i18n";

const testI18n = createI18n(i18nConfig);

function translatedText(key: string): string {
  return String(testI18n.global.t(key));
}

function mountApp() {
  return mount(App, {
    global: {
      plugins: [testI18n],
    },
  });
}

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
  window.sessionStorage.clear();
});

describe("App", () => {
  it("renders onboarding gating before chat becomes available", () => {
    const wrapper = mountApp();

    expect(wrapper.text()).toContain(translatedText("chatOnboarding.ready.title"));
    expect(wrapper.text()).toContain(translatedText("chatOnboarding.steps.runtime.title"));
    expect(
      wrapper.find(`input[placeholder="${translatedText("chat.inputPlaceholder")}"]`).exists()
    ).toBe(false);
  });

  it("toggles between configuration and onboarding gate", async () => {
    const wrapper = mountApp();

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");

    expect(
      wrapper.find(`input[placeholder="${translatedText("form.baseUrlPlaceholder")}"]`).exists()
    ).toBe(true);
    expect(
      wrapper.find(`input[placeholder="${translatedText("form.pairingCodePlaceholder")}"]`).exists()
    ).toBe(true);

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");

    expect(wrapper.text()).toContain(translatedText("chatOnboarding.steps.trust.title"));
  });

  it("pairs, gates on session start, and sends chat turns with bearer and session headers", async () => {
    vi.spyOn(crypto, "randomUUID")
      .mockReturnValueOnce("11111111-1111-4111-8111-111111111111")
      .mockReturnValueOnce("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa");

    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: false }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ paired: true, token: "zc_test_token" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      // fetchSessionList triggered by watcher when session becomes ready
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ sessions: [], total: 0 }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      // Stream endpoint — rejected so fallback to /webhook fires
      .mockResolvedValueOnce(new Response("", { status: 500 }))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            response: "Respuesta <b>ok</b>",
            session_id: "11111111-1111-4111-8111-111111111111",
          }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }
        )
      );

    const wrapper = mountApp();

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    await wrapper
      .get(`input[placeholder="${translatedText("form.pairingCodePlaceholder")}"]`)
      .setValue("123456");
    const pairButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("auth.pair"));
    if (!pairButton) {
      throw new Error("Expected pairing button to exist");
    }
    expect(pairButton.exists()).toBe(true);
    await pairButton.trigger("click");
    await flushPromises();

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    const startSessionButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("chat.startSession"));
    if (!startSessionButton) {
      throw new Error("Expected start session button to exist");
    }
    expect(startSessionButton.exists()).toBe(true);
    await startSessionButton.trigger("click");
    await flushPromises();

    const input = wrapper.get(`input[placeholder="${translatedText("chat.inputPlaceholder")}"]`);
    await input.setValue('Hola <script>alert("x")</script>');
    await wrapper.get("form").trigger("submit.prevent");
    await flushPromises();

    expect(fetchMock).toHaveBeenCalledTimes(7);

    function findCall(urlFragment: string): [RequestInfo | URL, RequestInit | undefined] {
      const match = fetchMock.mock.calls.find(([url]) => String(url).includes(urlFragment));
      if (!match) throw new Error(`No fetch call matching "${urlFragment}"`);
      return match as [RequestInfo | URL, RequestInit | undefined];
    }

    const [, pairInit] = findCall("/pair");
    expect((pairInit?.headers as Record<string, string>)["X-Pairing-Code"]).toBe("123456");

    const [, streamInit] = findCall("/web/chat/stream");
    expect((streamInit?.headers as Record<string, string>).Authorization).toBe(
      "Bearer zc_test_token"
    );
    expect((streamInit?.headers as Record<string, string>)["X-Session-Id"]).toBe(
      "11111111-1111-4111-8111-111111111111"
    );
    expect((streamInit?.headers as Record<string, string>)["X-Request-Id"]).toBe(
      "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
    );

    const [, webhookInit] = findCall("/webhook");
    expect((webhookInit?.headers as Record<string, string>).Authorization).toBe(
      "Bearer zc_test_token"
    );
    expect((webhookInit?.headers as Record<string, string>)["X-Session-Id"]).toBe(
      "11111111-1111-4111-8111-111111111111"
    );
    expect((webhookInit?.headers as Record<string, string>)["X-Idempotency-Key"]).toBe(
      "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
    );

    const chatMessages = wrapper.findAll('[data-testid="chat-message"]');
    expect(chatMessages).toHaveLength(3);
    expect(chatMessages[1]?.html()).toContain("&lt;script&gt;alert");
    expect(chatMessages[2]?.html()).toContain("&lt;b&gt;ok&lt;/b&gt;");
  });

  it("surfaces credential recovery when a chat turn is rejected", async () => {
    vi.spyOn(crypto, "randomUUID")
      .mockReturnValueOnce("22222222-2222-4222-8222-222222222222")
      .mockReturnValueOnce("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb");

    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      // fetchSessionList triggered by watcher when session becomes ready
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ sessions: [], total: 0 }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ error: "Unauthorized" }), {
          status: 401,
          headers: { "Content-Type": "application/json" },
        })
      );

    const wrapper = mountApp();
    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    await wrapper
      .get(`input[placeholder="${translatedText("form.bearerTokenPlaceholder")}"]`)
      .setValue("stale-token");
    const connectButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("auth.connect"));
    if (!connectButton) {
      throw new Error("Expected connect button to exist");
    }
    expect(connectButton.exists()).toBe(true);
    await connectButton.trigger("click");
    await flushPromises();

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    const sessionButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("chat.startSession"));
    if (!sessionButton) {
      throw new Error("Expected session button to exist");
    }
    expect(sessionButton.exists()).toBe(true);
    await sessionButton.trigger("click");
    await flushPromises();

    const input = wrapper.get(`input[placeholder="${translatedText("chat.inputPlaceholder")}"]`);
    await input.setValue("hola");
    await wrapper.get("form").trigger("submit.prevent");
    await flushPromises();

    expect(wrapper.text()).toContain(
      translatedText("chatOnboarding.recovery.credential_invalid.description")
    );
    expect(wrapper.text()).toContain(translatedText("chatOnboarding.steps.trust.title"));
  });

  it("accumulates streamed chunks instead of overwriting", async () => {
    vi.spyOn(crypto, "randomUUID")
      .mockReturnValueOnce("33333333-3333-4333-8333-333333333333")
      .mockReturnValueOnce("cccccccc-cccc-4ccc-8ccc-cccccccccccc");

    const sseBody =
      "event: chunk\ndata: Hello\n\nevent: chunk\ndata:  World\n\nevent: done\ndata: " +
      JSON.stringify({ message_id: "m1", session_id: "33333333-3333-4333-8333-333333333333" }) +
      "\n\n";

    fetchMock
      // Health check
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      // Connect
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      // fetchSessionList triggered by watcher when session becomes ready
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ sessions: [], total: 0 }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      // Stream endpoint — returns two chunks via SSE
      .mockResolvedValueOnce(
        new Response(sseBody, {
          status: 200,
          headers: { "Content-Type": "text/event-stream" },
        })
      );

    const wrapper = mountApp();

    // Connect with bearer token
    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    await wrapper
      .get(`input[placeholder="${translatedText("form.bearerTokenPlaceholder")}"]`)
      .setValue("valid-token");
    const connectButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("auth.connect"));
    if (!connectButton) throw new Error("Expected connect button");
    await connectButton.trigger("click");
    await flushPromises();

    // Start session
    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    const sessionButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("chat.startSession"));
    if (!sessionButton) throw new Error("Expected session button");
    await sessionButton.trigger("click");
    await flushPromises();

    // Send message
    const input = wrapper.get(`input[placeholder="${translatedText("chat.inputPlaceholder")}"]`);
    await input.setValue("test streaming");
    await wrapper.get("form").trigger("submit.prevent");
    await flushPromises();

    // The assistant message should contain concatenated chunks
    const chatMessages = wrapper.findAll('[data-testid="chat-message"]');
    const lastMessage = chatMessages[chatMessages.length - 1];
    expect(lastMessage?.text()).toContain("Hello World");
  });

  it("rejects persisted messages unless every entry has a finite integer id", async () => {
    window.sessionStorage.setItem("corvus.chat.session:%2Fapi", "resume-session-1");
    window.sessionStorage.setItem(
      "corvus-chat-messages-resume-session-1",
      JSON.stringify([
        { id: 1, role: "assistant", content: "valid" },
        { id: Number.POSITIVE_INFINITY, role: "user", content: "invalid" },
      ])
    );

    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "ok", paired: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ sessions: [], total: 0 }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

    const wrapper = mountApp();

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    await wrapper
      .get(`input[placeholder="${translatedText("form.bearerTokenPlaceholder")}"]`)
      .setValue("valid-token");
    const connectButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("auth.connect"));
    if (!connectButton) throw new Error("Expected connect button");
    await connectButton.trigger("click");
    await flushPromises();

    await wrapper.get('[data-testid="toggle-config"]').trigger("click");
    const resumeButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("chat.resumeSession"));
    if (!resumeButton) throw new Error("Expected resume button");
    await resumeButton.trigger("click");
    await flushPromises();

    const chatMessages = wrapper.findAll('[data-testid="chat-message"]');
    expect(chatMessages).toHaveLength(1);
    expect(chatMessages[0]?.text()).toContain("Corvus Agent");
    expect(wrapper.text()).not.toContain("invalid");
  });
});
