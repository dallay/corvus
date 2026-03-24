export const version = "0.4.0";

export {
  computeOnboardingSteps,
  type OnboardingProgress,
  type OnboardingStepConfig,
  type StepStatus,
} from "./onboarding";

export function resolvePublicUrl(value: string | undefined, fallback: string): string {
  const candidate = typeof value === "string" ? value.trim() : "";

  if (!candidate) {
    return fallback;
  }

  try {
    return new URL(candidate).toString().replace(/\/$/, "");
  } catch {
    return fallback;
  }
}
