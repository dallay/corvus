export const version = "0.4.0";

export {
  computeOnboardingSteps,
  type OnboardingProgress,
  type OnboardingStepConfig,
  type StepStatus,
} from "./onboarding";

/** Remove trailing slashes from a URL string without regex (avoids ReDoS / S5852). */
export function trimTrailingSlashes(url: string): string {
  let end = url.length;
  while (end > 0 && url[end - 1] === "/") {
    end--;
  }
  return url.slice(0, end);
}

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
