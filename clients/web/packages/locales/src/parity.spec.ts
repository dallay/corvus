import { describe, expect, it } from "vitest";
import en from "./en.json";
import es from "./es.json";

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function flatten(
  obj: Record<string, unknown>,
  prefix = "",
  result: Record<string, string> = Object.create(null) as Record<string, string>,
): Record<string, string> {
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (isRecord(value)) {
      flatten(value, fullKey, result);
    } else {
      result[fullKey] = String(value);
    }
  }
  return result;
}

function extractPlaceholders(text: string): string[] {
  const placeholders: string[] = [];
  let start = -1;
  let depth = 0;

  for (let index = 0; index < text.length; index += 1) {
    const codePoint = text.codePointAt(index);

    if (codePoint === 123) {
      if (start < 0) {
        start = index;
      }
      depth += 1;
      continue;
    }

    if (codePoint === 125 && start >= 0) {
      depth -= 1;

      if (depth <= 0) {
        if (index - start > 1) {
          placeholders.push(text.slice(start, index + 1));
        }
        start = -1;
        depth = 0;
      }
    }
  }

  return placeholders.sort((left, right) => left.localeCompare(right));
}

describe("Locale Parity Guard", () => {
  const flattenedEs = flatten(es);
  const flattenedEn = flatten(en);

  it("has identical sets of keys between Spanish and English", () => {
    const esKeys = Object.keys(flattenedEs).sort((left, right) => left.localeCompare(right));
    const enKeys = Object.keys(flattenedEn).sort((left, right) => left.localeCompare(right));

    expect(esKeys).toEqual(enKeys);
  });

  it("has matching placeholders for all shared keys", () => {
    for (const key of Object.keys(flattenedEs)) {
      if (Object.hasOwn(flattenedEn, key)) {
        const esPlaceholders = extractPlaceholders(flattenedEs[key]);
        const enPlaceholders = extractPlaceholders(flattenedEn[key]);

        expect(esPlaceholders, `Placeholder mismatch for key: ${key}`).toEqual(enPlaceholders);
      }
    }
  });

  it("preserves double-brace placeholders as distinct tokens", () => {
    const placeholders = extractPlaceholders("Hi {{name}} and {name}");

    expect(placeholders).toEqual(["{name}", "{{name}}"].sort((a, b) => a.localeCompare(b)));
  });

  it("preserves nested brace placeholder shapes", () => {
    const placeholders = extractPlaceholders("Value {outer{inner}} and {{user}}");

    expect(placeholders).toEqual(
      ["{outer{inner}}", "{{user}}"].sort((a, b) => a.localeCompare(b)),
    );
  });
});
