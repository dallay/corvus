export interface OnboardingProgress {
  runtimeConfirmed: boolean;
  trustEstablished: boolean;
  transportConnected: boolean;
  [key: string]: boolean;
}

export type StepStatus = "complete" | "blocked" | "current" | "pending";

export interface OnboardingStepConfig {
  key: string;
  titleKey: string;
  descriptionKey: string;
  status: StepStatus;
}

/**
 * Compute onboarding step statuses from progress and recovery state.
 */
export function computeOnboardingSteps(
  progress: OnboardingProgress,
  blockedRecovery: string | null,
  i18nPrefix: string,
  finalStepKey: string,
  finalProgressKey: string
): OnboardingStepConfig[] {
  return [
    {
      key: "runtime",
      titleKey: `${i18nPrefix}.runtime.title`,
      descriptionKey: `${i18nPrefix}.runtime.description`,
      status: progress.runtimeConfirmed
        ? "complete"
        : blockedRecovery === "runtime_unavailable" || blockedRecovery === "transport_unavailable"
          ? "blocked"
          : "current",
    },
    {
      key: "trust",
      titleKey: `${i18nPrefix}.trust.title`,
      descriptionKey: `${i18nPrefix}.trust.description`,
      status: progress.trustEstablished
        ? "complete"
        : !progress.runtimeConfirmed
          ? "pending"
          : blockedRecovery === "trust_input_invalid" ||
              blockedRecovery === "trust_input_expired" ||
              blockedRecovery === "credential_missing" ||
              blockedRecovery === "credential_invalid"
            ? "blocked"
            : "current",
    },
    {
      key: "connect",
      titleKey: `${i18nPrefix}.connect.title`,
      descriptionKey: `${i18nPrefix}.connect.description`,
      status: progress.transportConnected
        ? "complete"
        : !progress.trustEstablished
          ? "pending"
          : blockedRecovery === "paired_but_not_connected"
            ? "blocked"
            : "current",
    },
    {
      key: finalStepKey,
      titleKey: `${i18nPrefix}.${finalStepKey}.title`,
      descriptionKey: `${i18nPrefix}.${finalStepKey}.description`,
      status: progress[finalProgressKey]
        ? "complete"
        : !progress.transportConnected
          ? "pending"
          : "current",
    },
  ];
}
