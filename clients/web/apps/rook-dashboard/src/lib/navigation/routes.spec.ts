import { describe, expect, it } from "vitest";

import { normalizeHashRoute, toHashRoute } from "./routes";

describe("routes", () => {
  it("defaults unknown hashes to overview", () => {
    expect(normalizeHashRoute("")).toBe("overview");
    expect(normalizeHashRoute("#/something-else")).toBe("overview");
  });

  it("parses supported hash routes", () => {
    expect(normalizeHashRoute("#/overview")).toBe("overview");
    expect(normalizeHashRoute("#/accounts")).toBe("accounts");
    expect(normalizeHashRoute("#/pools")).toBe("pools");
    expect(normalizeHashRoute("#/routes")).toBe("routes");
    expect(normalizeHashRoute("#/health")).toBe("health");
    expect(normalizeHashRoute("#/usage")).toBe("usage");
    expect(normalizeHashRoute("#/settings")).toBe("settings");
  });

  it("keeps deferred #594 areas out of the supported route set", () => {
    expect(normalizeHashRoute("#/logs")).toBe("overview");
    expect(normalizeHashRoute("#/backups")).toBe("overview");
  });

  it("serializes routes back to hash form", () => {
    expect(toHashRoute("overview")).toBe("#/overview");
    expect(toHashRoute("accounts")).toBe("#/accounts");
    expect(toHashRoute("pools")).toBe("#/pools");
    expect(toHashRoute("routes")).toBe("#/routes");
    expect(toHashRoute("health")).toBe("#/health");
    expect(toHashRoute("usage")).toBe("#/usage");
    expect(toHashRoute("settings")).toBe("#/settings");
  });
});
