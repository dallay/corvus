import { describe, expect, it } from "vitest";

import { isPlaywrightFsAllowMode } from "./playwrightEnv";

describe("isPlaywrightFsAllowMode", () => {
  it("returns false for regular dev env", () => {
    expect(isPlaywrightFsAllowMode({})).toBe(false);
  });

  it("returns true when PLAYWRIGHT flag is enabled", () => {
    expect(isPlaywrightFsAllowMode({ PLAYWRIGHT: "true" })).toBe(true);
  });

  it("returns true for legacy NODE_ENV test mode", () => {
    expect(isPlaywrightFsAllowMode({ NODE_ENV: "test" })).toBe(true);
  });
});
