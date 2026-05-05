import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { computed, ref } from "vue";
import { createI18n } from "vue-i18n";

import ChatWorkspace from "@/components/chat/ChatWorkspace.vue";
import type { useConfig } from "@/composables/useConfig";
import { i18nConfig } from "@/i18n";
import { expectNoAxeViolations } from "@/test/runAxe";

const testI18n = createI18n(i18nConfig);
const mountedWrappers: Array<ReturnType<typeof mount>> = [];

function translatedText(key: string): string {
  return String(testI18n.global.t(key));
}

/**
 * Creates a minimal mock of useConfig return type that ChatWorkspace expects
 * as its `config` prop.
 */
function createMockConfig(
  overrides?: Partial<ReturnType<typeof useConfig>>
): ReturnType<typeof useConfig> {
  const baseUrl = ref("/api");
  const bearerToken = ref("");
  const errorMessage = ref("");
  const statusMessage = ref("");
  const isOperatorReady = computed(() => !!bearerToken.value);

  return {
    baseUrl,
    pairingCode: ref(""),
    bearerToken,
    loading: ref(false),
    statusMessage,
    errorMessage,
    form: {} as ReturnType<typeof useConfig>["form"],
    canSave: computed(() => false),
    sectionSaving: {} as ReturnType<typeof useConfig>["sectionSaving"],
    memoryBackendOptions: ref([]),
    observabilityBackendOptions: ref([]),
    runtimeKindOptions: ref([]),
    autonomyLevelOptions: ref([]),
    quickPairState: ref("idle" as const),
    onboardingState: ref({
      surfaceId: "web_dashboard" as const,
      state: "trust_pending" as const,
      trustMode: "http_paired" as const,
      transportMode: "http_gateway" as const,
      recoveryKind: null,
      canRetry: false,
      canResume: false,
      persistsPairingCode: false as const,
      persistsBearerToken: false,
    }),
    lastTransitionLabel: ref(null),
    currentRecoveryLabel: computed(() => null),
    onboardingSteps: computed(() => []),
    isOperatorReady,
    pairGateway: vi.fn().mockResolvedValue(false),
    connectGateway: vi.fn().mockResolvedValue(false),
    saveSection: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  } as ReturnType<typeof useConfig>;
}

function mountWorkspace(configOverrides?: Partial<ReturnType<typeof useConfig>>) {
  const config = createMockConfig(configOverrides);
  const wrapper = mount(ChatWorkspace, {
    attachTo: document.body,
    props: { config },
    global: {
      plugins: [testI18n],
    },
  });
  mountedWrappers.push(wrapper);
  return wrapper;
}

function mountReadyWorkspace() {
  const bearerToken = ref("valid-token");
  const isOperatorReady = computed(() => true);
  return {
    wrapper: mountWorkspace({
      bearerToken,
      isOperatorReady,
    }),
    bearerToken,
  };
}

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
  window.sessionStorage.clear();
});

afterEach(() => {
  vi.useRealTimers();
  while (mountedWrappers.length > 0) {
    mountedWrappers.pop()?.unmount();
  }
});

describe("ChatWorkspace", () => {
  it("renders session gate when operator is not ready", () => {
    const wrapper = mountWorkspace();

    // Should show the session gate, not the chat input
    expect(wrapper.find(".chat-gate").exists()).toBe(true);
    expect(
      wrapper.find(`input[placeholder="${translatedText("chat.inputPlaceholder")}"]`).exists()
    ).toBe(false);
  });

  it("shows start session and resume session buttons when operator is ready", () => {
    const { wrapper } = mountReadyWorkspace();

    expect(wrapper.find(".chat-gate").exists()).toBe(true);
    const buttons = wrapper.findAll("button");
    const startButton = buttons.find(
      (button) => button.text() === translatedText("chat.startSession")
    );
    const resumeButton = buttons.find(
      (button) => button.text() === translatedText("chat.resumeSession")
    );
    expect(startButton?.exists()).toBe(true);
    expect(resumeButton?.exists()).toBe(true);
  });

  it("associates the prompt with its disclaimer after entering chat", async () => {
    const { wrapper: readyWrapper } = mountReadyWorkspace();
    const startButton = readyWrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("chat.startSession"));
    await startButton?.trigger("click");
    await flushPromises();

    const promptInput = readyWrapper.get("#chat-prompt-input");
    expect(promptInput.attributes("aria-describedby")).toBe("chat-input-disclaimer");
    expect(readyWrapper.find('label[for="chat-prompt-input"]').classes()).toContain("sr-only");
  });

  it("has no obvious axe violations for the onboarding gate", async () => {
    const wrapper = mountWorkspace();

    expect(wrapper.find(".chat-gate").exists()).toBe(true);
    expect(wrapper.get(".chat-gate h2").text()).toContain("sesión");
    await expectNoAxeViolations(wrapper.get(".chat-gate").element, {
      rules: {
        region: { enabled: false },
      },
    });
  });

  it("starts a new session and shows chat input when start session is clicked", async () => {
    vi.spyOn(crypto, "randomUUID").mockReturnValueOnce("11111111-1111-4111-8111-111111111111");

    // Mock the session list fetch that happens when session becomes ready
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ sessions: [], total: 0 }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );

    const { wrapper } = mountReadyWorkspace();

    const startButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("chat.startSession"));
    expect(startButton?.exists()).toBe(true);
    await startButton?.trigger("click");
    await flushPromises();

    expect(
      wrapper.find(`input[placeholder="${translatedText("chat.inputPlaceholder")}"]`).exists()
    ).toBe(true);
    expect(document.activeElement).toBe(wrapper.get("#chat-prompt-input").element);
  });

  it("announces and focuses the prompt when switching sessions", async () => {
    fetchMock.mockResolvedValue(
      new Response(
        JSON.stringify({
          sessions: [
            {
              id: "11111111-1111-4111-8111-111111111111",
              started_at: "2026-03-28T10:00:00Z",
              ended_at: null,
              message_count: 5,
              last_activity: "2026-03-28T11:00:00Z",
            },
            {
              id: "session-2",
              started_at: "2026-03-27T10:00:00Z",
              ended_at: null,
              message_count: 2,
              last_activity: "2026-03-27T10:30:00Z",
            },
          ],
          total: 2,
        }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }
      )
    );
    vi.spyOn(crypto, "randomUUID").mockReturnValueOnce("11111111-1111-4111-8111-111111111111");

    const { wrapper } = mountReadyWorkspace();
    const startButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("chat.startSession"));
    await startButton?.trigger("click");
    await flushPromises();

    const sidebar = wrapper.findComponent({ name: "SessionSidebar" });
    sidebar.vm.$emit("switch-session", "session-2");
    await flushPromises();
    await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
    await flushPromises();

    expect(wrapper.text()).toContain("session-2");
    expect(wrapper.findAll("output")[0]?.text()).toContain("session-2");
    expect(document.activeElement).toBe(wrapper.get("#chat-prompt-input").element);
  });

  it("announces approval decisions and restores focus to the prompt", async () => {
    const { wrapper } = mountReadyWorkspace();

    const startButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("chat.startSession"));
    await startButton?.trigger("click");
    await flushPromises();

    fetchMock.mockResolvedValueOnce(new Response("", { status: 500 }));
    fetchMock.mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          code: "approval_required",
          tool: "file_write",
          reason: "Needs confirmation",
          session_id: "11111111-1111-4111-8111-111111111111",
        }),
        {
          status: 403,
          headers: { "Content-Type": "application/json" },
        }
      )
    );

    const promptInput = wrapper.get("#chat-prompt-input");
    await promptInput.setValue("approve this");
    await wrapper.get("form").trigger("submit.prevent");
    await flushPromises();

    const approveButton = wrapper.get('[data-testid="btn-approve"]');
    expect(document.activeElement).toBe(approveButton.element);

    await approveButton.trigger("click");
    await flushPromises();
    await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
    await flushPromises();

    const liveOutputs = wrapper.findAll("output");
    expect(liveOutputs[1]?.text()).toContain(translatedText("chat.approve"));
    expect(document.activeElement).toBe(wrapper.get("#chat-prompt-input").element);
  });

  it("sends chat message and renders response", async () => {
    vi.spyOn(crypto, "randomUUID")
      .mockReturnValueOnce("22222222-2222-4222-8222-222222222222")
      .mockReturnValueOnce("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb");

    // Mock session list fetch
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ sessions: [], total: 0 }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );

    const { wrapper } = mountReadyWorkspace();

    // Start session
    const startButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("chat.startSession"));
    await startButton?.trigger("click");
    await flushPromises();

    // Mock stream endpoint (will fail, triggering fallback)
    fetchMock.mockResolvedValueOnce(new Response("", { status: 500 }));
    // Mock webhook fallback
    fetchMock.mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          response: "Respuesta ok",
          session_id: "22222222-2222-4222-8222-222222222222",
        }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }
      )
    );

    const input = wrapper.get(`input[placeholder="${translatedText("chat.inputPlaceholder")}"]`);
    await input.setValue("Hola");
    await wrapper.get("form").trigger("submit.prevent");
    await flushPromises();

    const chatMessages = wrapper.findAll('[data-testid="chat-message"]');
    // Welcome message + user message + assistant response = 3
    expect(chatMessages.length).toBeGreaterThanOrEqual(3);
  });

  it("accumulates streamed chunks instead of overwriting", async () => {
    vi.spyOn(crypto, "randomUUID")
      .mockReturnValueOnce("33333333-3333-4333-8333-333333333333")
      .mockReturnValueOnce("cccccccc-cccc-4ccc-8ccc-cccccccccccc");

    const sseBody =
      "event: chunk\ndata: Hello\n\nevent: chunk\ndata:  World\n\nevent: done\ndata: " +
      JSON.stringify({ message_id: "m1", session_id: "33333333-3333-4333-8333-333333333333" }) +
      "\n\n";

    // Mock session list fetch
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ sessions: [], total: 0 }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );

    const { wrapper } = mountReadyWorkspace();

    // Start session
    const startButton = wrapper
      .findAll("button")
      .find((button) => button.text() === translatedText("chat.startSession"));
    await startButton?.trigger("click");
    await flushPromises();

    // Stream endpoint returns SSE chunks
    fetchMock.mockResolvedValueOnce(
      new Response(sseBody, {
        status: 200,
        headers: { "Content-Type": "text/event-stream" },
      })
    );

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

    // Mock session list fetch
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ sessions: [], total: 0 }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );

    const { wrapper } = mountReadyWorkspace();

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
