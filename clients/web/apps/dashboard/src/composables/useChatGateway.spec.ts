import { beforeEach, describe, expect, it, vi } from "vitest";
import { computed, ref } from "vue";

import { useChatGateway } from "@/composables/useChatGateway";
import type { useConfig } from "@/composables/useConfig";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

/**
 * Creates a minimal mock of the useConfig return type, which is what
 * useChatGateway expects as its first argument.
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

describe("useChatGateway", () => {
  it("adapts useConfig into the ChatGateway interface", () => {
    const config = createMockConfig();
    config.bearerToken.value = "my-token";
    const gateway = useChatGateway(config, (key: string) => key);

    expect(gateway.bearerToken.value).toBe("my-token");
    expect(gateway.baseUrl.value).toBe("/api");
    expect(gateway.isGatewayReady.value).toBe(true);
  });

  it("reports not ready when operator is not ready", () => {
    const config = createMockConfig();
    const gateway = useChatGateway(config, (key: string) => key);

    expect(gateway.isGatewayReady.value).toBe(false);
  });

  it("normalizeBaseUrl trims trailing slashes", () => {
    const config = createMockConfig();
    config.baseUrl.value = "http://localhost:3000/api/";
    const gateway = useChatGateway(config, (key: string) => key);

    expect(gateway.normalizeBaseUrl()).toBe("http://localhost:3000/api");
  });

  it("normalizeBaseUrl defaults to /api when empty", () => {
    const config = createMockConfig();
    config.baseUrl.value = "";
    const gateway = useChatGateway(config, (key: string) => key);

    expect(gateway.normalizeBaseUrl()).toBe("/api");
  });

  it("gatewayUrl joins path to base URL correctly", () => {
    const config = createMockConfig();
    config.baseUrl.value = "https://remote.example.com/api";
    const gateway = useChatGateway(config, (key: string) => key);

    const url = gateway.gatewayUrl("/health");
    expect(url).toBe("https://remote.example.com/api/health");
  });

  it("authHeaders includes bearer token and Content-Type by default", () => {
    const config = createMockConfig();
    config.bearerToken.value = "my-token";
    const gateway = useChatGateway(config, (key: string) => key);

    const headers = gateway.authHeaders();

    expect(headers["Content-Type"]).toBe("application/json");
    expect(headers.Authorization).toBe("Bearer my-token");
  });

  it("authHeaders omits Content-Type when includeJsonContentType is false", () => {
    const config = createMockConfig();
    config.bearerToken.value = "my-token";
    const gateway = useChatGateway(config, (key: string) => key);

    const headers = gateway.authHeaders(false);

    expect(headers["Content-Type"]).toBeUndefined();
    expect(headers.Authorization).toBe("Bearer my-token");
  });

  it("authHeaders includes webhook secret when set", () => {
    const config = createMockConfig();
    config.bearerToken.value = "my-token";
    const gateway = useChatGateway(config, (key: string) => key);
    gateway.webhookSecret.value = "my-secret";

    const headers = gateway.authHeaders();

    expect(headers["X-Webhook-Secret"]).toBe("my-secret");
  });

  it("createIdempotencyKey returns a UUID-like string", () => {
    const gateway = useChatGateway(createMockConfig(), (key: string) => key);
    const key = gateway.createIdempotencyKey();
    // Should be a valid UUID or fallback format
    expect(key).toBeTruthy();
    expect(typeof key).toBe("string");
  });

  it("createIdempotencyKey falls back to timestamp-based key when crypto.randomUUID is unavailable", () => {
    const originalCrypto = globalThis.crypto;
    vi.stubGlobal("crypto", { randomUUID: undefined });

    const gateway = useChatGateway(createMockConfig(), (key: string) => key);
    const key = gateway.createIdempotencyKey();

    expect(key).toMatch(/^chat-\d+-[a-z0-9]+$/);

    vi.stubGlobal("crypto", originalCrypto);
  });

  describe("getSessionList", () => {
    it("calls correct URL with auth headers and parses response", async () => {
      const config = createMockConfig();
      config.bearerToken.value = "my-token";
      const gateway = useChatGateway(config, (key: string) => key);

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
              {
                id: "s2",
                started_at: "2026-01-02",
                ended_at: "2026-01-03",
                message_count: 3,
                last_activity: "2026-01-03",
              },
            ],
            total: 2,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        )
      );

      const result = await gateway.getSessionList();

      expect(result.sessions).toHaveLength(2);
      expect(result.sessions[0]?.id).toBe("s1");
      expect(result.total).toBe(2);

      const [url, init] = fetchMock.mock.calls[0] ?? [];
      expect(String(url)).toContain("/session/list");
      expect(String(url)).toContain("limit=20");
      expect(String(url)).toContain("offset=0");
      expect((init?.headers as Record<string, string>).Authorization).toBe("Bearer my-token");
    });

    it("returns empty sessions on 404", async () => {
      const config = createMockConfig();
      const gateway = useChatGateway(config, (key: string) => key);

      fetchMock.mockResolvedValueOnce(new Response(null, { status: 404 }));

      const result = await gateway.getSessionList();

      expect(result.sessions).toEqual([]);
      expect(result.total).toBe(0);
    });

    it("returns empty sessions on network error", async () => {
      const config = createMockConfig();
      const gateway = useChatGateway(config, (key: string) => key);

      fetchMock.mockRejectedValueOnce(new Error("network failure"));

      const result = await gateway.getSessionList();

      expect(result.sessions).toEqual([]);
      expect(result.total).toBe(0);
    });

    it("passes limit and offset params through to the gateway", async () => {
      const config = createMockConfig();
      const gateway = useChatGateway(config, (key: string) => key);

      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify({ sessions: [], total: 0 }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      await gateway.getSessionList(10, 20);

      const [url] = fetchMock.mock.calls[0] ?? [];
      expect(String(url)).toContain("limit=10");
      expect(String(url)).toContain("offset=20");
    });
  });

  it("markCredentialInvalid sets error message on config", () => {
    const config = createMockConfig();
    const gateway = useChatGateway(config, (key: string) => key);

    gateway.markCredentialInvalid();

    expect(config.errorMessage.value).toBe("auth.credentialInvalid");
  });

  it("markPairedButNotConnected sets error message on config", () => {
    const config = createMockConfig();
    const gateway = useChatGateway(config, (key: string) => key);

    gateway.markPairedButNotConnected();

    expect(config.errorMessage.value).toBe("chatOnboarding.transportDisconnected");
  });
});
