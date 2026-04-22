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
  });

  it("serializes routes back to hash form", () => {
    expect(toHashRoute("overview")).toBe("#/overview");
    expect(toHashRoute("accounts")).toBe("#/accounts");
  });
});
