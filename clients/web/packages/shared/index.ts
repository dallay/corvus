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

/**
 * Parse and validate a gateway URL. Returns a sanitised URL object or null
 * when the input is empty, malformed, or uses a non-HTTP(S) protocol.
 */
export function validateGatewayUrl(rawUrl: string): URL | null {
  const trimmed = rawUrl.trim();
  if (!trimmed) return null;

  let parsed: URL;
  try {
    parsed = new URL(trimmed);
  } catch {
    return null;
  }

  if (!["http:", "https:"].includes(parsed.protocol)) return null;

  parsed.pathname = trimTrailingSlashes(parsed.pathname);
  parsed.search = "";
  parsed.hash = "";
  return parsed;
}

export function resolvePublicUrl(value: string | undefined, fallback: string): string {
  const candidate = typeof value === "string" ? value.trim() : "";

  if (!candidate) {
    return fallback;
  }

  try {
    return trimTrailingSlashes(new URL(candidate).toString());
  } catch {
    return fallback;
  }
}
