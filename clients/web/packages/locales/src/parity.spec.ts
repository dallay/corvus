import { describe, expect, it } from "vitest";
import en from "./en.json";
import es from "./es.json";

function flatten(obj: Record<string, unknown>, prefix = ""): Record<string, string> {
  let result: Record<string, string> = {};
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (value && typeof value === "object" && !Array.isArray(value)) {
      result = {
        ...result,
        ...flatten(value as Record<string, unknown>, fullKey),
      };
    } else {
      result[fullKey] = typeof value === "object" ? JSON.stringify(value) : String(value);
    }
  }
  return result;
}

function extractPlaceholders(text: string): string[] {
  const matches = text.match(/\{[^}]+\}/g) || [];
  return matches.sort((a, b) => a.localeCompare(b));
}

describe("Locale Parity Guard", () => {
  const flattenedEs = flatten(es as unknown as Record<string, unknown>);
  const flattenedEn = flatten(en as unknown as Record<string, unknown>);

  it("has identical sets of keys between Spanish and English", () => {
    const esKeys = Object.keys(flattenedEs).sort((a, b) => a.localeCompare(b));
    const enKeys = Object.keys(flattenedEn).sort((a, b) => a.localeCompare(b));

    expect(esKeys).toEqual(enKeys);
  });

  it("has matching placeholders for all shared keys", () => {
    for (const key of Object.keys(flattenedEs)) {
      if (flattenedEn[key]) {
        const esPlaceholders = extractPlaceholders(flattenedEs[key]);
        const enPlaceholders = extractPlaceholders(flattenedEn[key]);

        expect(esPlaceholders, `Placeholder mismatch for key: ${key}`).toEqual(enPlaceholders);
      }
    }
  });
});
