function readDefaultEnv(): Record<string, string | undefined> {
  return (
    (globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env ?? {}
  );
}

export function isPlaywrightFsAllowMode(
  env: Record<string, string | undefined> = readDefaultEnv()
) {
  return env.NODE_ENV === "test" || env.PLAYWRIGHT === "true";
}
