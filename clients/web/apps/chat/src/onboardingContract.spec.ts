import { translations } from "@corvus/locales";
import { describe, expect, it } from "vitest";

import {
  useGateway,
  webChatIntentSelection,
  webChatOnboardingRecoveryLabel,
} from "@/composables/useGateway";
import mobileChatWorkspace from "../../../../../clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt?raw";
import clientSurfacesSpec from "../../../../../openspec/specs/client-surfaces/spec.md?raw";
import dashboardSpec from "../../../../../openspec/specs/dashboard/spec.md?raw";
import onboardingSpec from "../../../../../openspec/specs/onboarding/spec.md?raw";

describe("onboarding contract evidence", () => {
  it("maps shared onboarding steps to the same user outcomes across web surfaces", () => {
    const chat = useGateway((key: string) => key);

    expect(chat.onboardingState.value.surfaceId).toBe("web_chat");
    expect(chat.onboardingState.value.transportMode).toBe("http_gateway");
    expect(chat.onboardingState.value.trustMode).toBe("http_paired");

    expect(chat.onboardingSteps.value.map((step) => step.key)).toEqual([
      "runtime",
      "trust",
      "connect",
      "ready",
    ]);

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
    const chatIntent = webChatIntentSelection();

    expect(chatIntent).toEqual({
      surfaceId: "web_chat",
      intent: "end_user",
      trustMode: "http_paired",
      transportMode: "http_gateway",
      requiresSessionEntry: true,
    });
  });

  it("keeps broader cross-surface same-outcome parity through shared web adapters", () => {
    const chat = useGateway((key: string) => key);

    expect(chat.onboardingSteps.value.slice(0, 3).map((step) => step.titleKey)).toEqual([
      "chatOnboarding.steps.runtime.title",
      "chatOnboarding.steps.trust.title",
      "chatOnboarding.steps.connect.title",
    ]);
    expect(chat.onboardingSteps.value.slice(0, 3).map((step) => step.descriptionKey)).toEqual([
      "chatOnboarding.steps.runtime.description",
      "chatOnboarding.steps.trust.description",
      "chatOnboarding.steps.connect.description",
    ]);
    expect(translations.en.chatOnboarding.steps.runtime.title).toBe(
      translations.en.onboarding.steps.runtime.title
    );
    expect(translations.en.chatOnboarding.steps.trust.title).toBe(
      translations.en.onboarding.steps.trust.title
    );
    expect(translations.en.chatOnboarding.steps.connect.title).toBe(
      translations.en.onboarding.steps.connect.title
    );

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
      expect(webChatOnboardingRecoveryLabel(recovery)).toBe(recovery);
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

  it("keeps web and mobile recovery labels comparable through normalized product taxonomy", () => {
    expect(webChatOnboardingRecoveryLabel("runtime_unavailable")).toBe("runtime_unavailable");
    expect(webChatOnboardingRecoveryLabel("paired_but_not_connected")).toBe(
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
