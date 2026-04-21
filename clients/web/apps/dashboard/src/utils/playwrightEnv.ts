export function isPlaywrightFsAllowMode(env: Record<string, string | undefined> = process.env) {
  return env.NODE_ENV === "test" || env.PLAYWRIGHT === "true";
}
