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

function resolveStepStatus(
  isComplete: boolean,
  isPreviousStepComplete: boolean,
  isBlocked: boolean
): StepStatus {
  if (isComplete) return "complete";
  if (!isPreviousStepComplete) return "pending";
  if (isBlocked) return "blocked";
  return "current";
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
      status: resolveStepStatus(
        progress.runtimeConfirmed,
        true,
        blockedRecovery === "runtime_unavailable" || blockedRecovery === "transport_unavailable"
      ),
    },
    {
      key: "trust",
      titleKey: `${i18nPrefix}.trust.title`,
      descriptionKey: `${i18nPrefix}.trust.description`,
      status: resolveStepStatus(
        progress.trustEstablished,
        progress.runtimeConfirmed,
        blockedRecovery === "trust_input_invalid" ||
          blockedRecovery === "trust_input_expired" ||
          blockedRecovery === "credential_missing" ||
          blockedRecovery === "credential_invalid"
      ),
    },
    {
      key: "connect",
      titleKey: `${i18nPrefix}.connect.title`,
      descriptionKey: `${i18nPrefix}.connect.description`,
      status: resolveStepStatus(
        progress.transportConnected,
        progress.trustEstablished,
        blockedRecovery === "paired_but_not_connected"
      ),
    },
    {
      key: finalStepKey,
      titleKey: `${i18nPrefix}.${finalStepKey}.title`,
      descriptionKey: `${i18nPrefix}.${finalStepKey}.description`,
      status: resolveStepStatus(progress[finalProgressKey], progress.transportConnected, false),
    },
  ];
}
