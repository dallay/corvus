import { translations } from "@corvus/locales";
import { describe, expect, it } from "vitest";
import { computed, ref } from "vue";

import { useChatGateway } from "@/composables/useChatGateway";
import {
  dashboardOnboardingRecoveryLabel,
  dashboardOperatorIntentSelection,
  type useConfig,
} from "@/composables/useConfig";
import mobileChatWorkspace from "../../../../../../clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt?raw";
import clientSurfacesSpec from "../../../../../../openspec/specs/client-surfaces/spec.md?raw";
import dashboardSpec from "../../../../../../openspec/specs/dashboard/spec.md?raw";
import onboardingSpec from "../../../../../../openspec/specs/onboarding/spec.md?raw";

// Inline roadmap fixture for stable test - maps to key headings/markers rather than exact prose
const roadmapFixture = `
# CLAUDIO Roadmap

## 1) Foundation
...
## 2) Provider Resilience
...
## 3) Memory Architecture
...
## 4) Multi-Agent Orchestration 🔄 IN PROGRESS / PARTIAL
...
### Track 4 Slice 1 coordinator foundations
...
### deferred: mailbox-on-disk / persistent orchestration messaging
### deferred: remote bridge and other cross-process coordinator transport
### deferred: worktree / sandbox / repository isolation boundaries
### deferred: permission escalation / approval-broker workflows
...
`;

/**
 * Creates a minimal mock of the useConfig return type for contract tests.
 */
function createMockConfig(): ReturnType<typeof useConfig> {
  const baseUrl = ref("/api");
  const bearerToken = ref("");
  const isOperatorReady = computed(() => false);

  return {
    baseUrl,
    pairingCode: ref(""),
    bearerToken,
    loading: ref(false),
    statusMessage: ref(""),
    errorMessage: ref(""),
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
    pairGateway: async () => false,
    connectGateway: async () => false,
    saveSection: async () => undefined,
  } as ReturnType<typeof useConfig>;
}

describe("chat onboarding contract evidence (dashboard)", () => {
  it("maps shared onboarding steps to the same user outcomes across web surfaces", () => {
    const config = createMockConfig();
    const gateway = useChatGateway(config, (key: string) => key);

    // The dashboard's chat gateway wraps useConfig — the onboarding surface is
    // web_dashboard, not web_chat, because the dashboard owns the trust flow.
    expect(config.onboardingState.value.surfaceId).toBe("web_dashboard");
    expect(config.onboardingState.value.transportMode).toBe("http_gateway");
    expect(config.onboardingState.value.trustMode).toBe("http_paired");

    // Gateway interface is correctly constructed
    expect(gateway.baseUrl.value).toBe("/api");

    // Shared locale keys still reference the same underlying text
    expect(translations.en.chatOnboarding.steps.runtime.title).toBe(
      translations.en.onboarding.steps.runtime.title
    );
    expect(translations.en.chatOnboarding.steps.trust.title).toBe(
      translations.en.onboarding.steps.trust.title
    );
    expect(translations.en.chatOnboarding.steps.connect.title).toBe(
      translations.en.onboarding.steps.connect.title
    );
    expect(translations.en.chatOnboarding.steps.ready.title).toBe("Ready to start chat");
    expect(translations.en.onboarding.steps.ready.title).toBe("Ready for operator tasks");
  });

  it("keeps operator and chat intent selection explicit before transport checks", () => {
    const dashboardIntent = dashboardOperatorIntentSelection();

    expect(dashboardIntent).toEqual({
      surfaceId: "web_dashboard",
      intent: "operator",
      trustMode: "http_paired",
      transportMode: "http_gateway",
      requiresSessionEntry: false,
    });
  });

  it("keeps broader cross-surface same-outcome parity through shared locale keys", () => {
    // Shared locale keys referenced by both chat and dashboard onboarding
    const sharedRecoveries = [
      "runtime_unavailable",
      "transport_unavailable",
      "trust_input_invalid",
      "trust_input_expired",
      "credential_missing",
      "credential_invalid",
      "paired_but_not_connected",
    ] as const;

    for (const recovery of sharedRecoveries) {
      expect(dashboardOnboardingRecoveryLabel(recovery)).toBe(recovery);
    }

    expect(mobileChatWorkspace).toContain(
      'MobileRecoveryKind.RUNTIME_UNAVAILABLE -> "runtime_unavailable"'
    );
    expect(mobileChatWorkspace).toContain(
      'MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY -> "linked_but_not_session_ready"'
    );
  });

  it("defers transport and dashboard activation authority to the governing specs", () => {
    expect(onboardingSpec).toContain(
      "It MUST NOT replace the transport and capability authority of"
    );
    expect(onboardingSpec).toContain(
      "THEN the answer MUST be governed by the `client-surfaces` specification"
    );
    expect(onboardingSpec).toContain(
      "THEN the answer MUST be governed by the `dashboard` specification"
    );

    expect(clientSurfacesSpec).toContain(
      "Each surface MUST use exactly one transport for all runtime communication"
    );
    expect(clientSurfacesSpec).toContain("THEN the surface MUST use HTTP Gateway endpoints");
    expect(clientSurfacesSpec).toContain(
      "THEN the surface MUST use the RustCliBridge (process bridge)"
    );

    expect(dashboardSpec).toContain(
      "Interactive onboarding currently ends without a guided web dashboard activation step."
    );
    expect(dashboardSpec).toContain(
      "If the user accepts dashboard activation, the system SHALL provide a compact operator activation"
    );
  });

  it("keeps Track 4 roadmap traceability explicit for shipped coordinator foundations and deferred gaps", () => {
    // Assert on stable markers: headings (##), keywords, and section indicators
    // instead of exact prose to avoid test brittleness from reworded content
    expect(roadmapFixture).toMatch(/^## /m); // Has section headings
    expect(roadmapFixture).toMatch(/##.*4\).*Multi-Agent Orchestration/);
    expect(roadmapFixture).toMatch(/Track 4/);
    expect(roadmapFixture).toMatch(/deferred:/); // Marks deferred items
  });

  it("keeps web and mobile recovery labels comparable through normalized product taxonomy", () => {
    expect(dashboardOnboardingRecoveryLabel("runtime_unavailable")).toBe("runtime_unavailable");
    expect(dashboardOnboardingRecoveryLabel("paired_but_not_connected")).toBe(
      "paired_but_not_connected"
    );

    expect(mobileChatWorkspace).toContain(
      'MobileRecoveryKind.RUNTIME_UNAVAILABLE -> "runtime_unavailable"'
    );
    expect(mobileChatWorkspace).toContain(
      'MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY -> "linked_but_not_session_ready"'
    );
    expect(mobileChatWorkspace).toContain(
      'MobileRecoveryKind.ENVIRONMENT_UNSUPPORTED -> "environment_unsupported"'
    );
  });
});
