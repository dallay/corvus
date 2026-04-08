import { describe, expect, it } from "vitest";
import en from "./en.json";
import es from "./es.json";

const requiredSessionAndMemoryKeys = [
  "chat.newChat",
  "session.history",
  "session.justNow",
  "session.minutesAgo",
  "session.hoursAgo",
  "session.daysAgo",
  "session.messageCount",
  "session.sidebarLabel",
  "session.expand",
  "session.collapse",
  "session.noHistory",
  "memory.statsLoading",
  "memory.statTotalEntries",
  "memory.statTotalSessions",
  "memory.statActiveSessions",
  "memory.statBackend",
  "memory.cerebroConfigured",
  "memory.cerebroNotConfigured",
  "memory.statCerebro",
  "memory.statByCategory",
  "memory.loading",
  "memory.empty",
  "memory.colKey",
  "memory.colCategory",
  "memory.colTimestamp",
  "memory.colSessionId",
  "memory.colContent",
  "memory.colActions",
  "memory.delete",
  "memory.confirmDelete",
  "memory.confirmDeletePrompt",
  "memory.confirmYes",
  "memory.confirmNo",
  "memory.filterCategory",
  "memory.filterAll",
  "memory.filterSessionId",
  "memory.sessionIdPlaceholder",
  "memory.filterSearch",
  "memory.searchPlaceholder",
  "pagination.prev",
  "pagination.page",
  "pagination.total",
  "pagination.next",
] as const;

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

  it("includes required session sidebar and memory locale keys", () => {
    for (const key of requiredSessionAndMemoryKeys) {
      expect(flattenedEn, `Missing English locale key: ${key}`).toHaveProperty(key);
      expect(flattenedEs, `Missing Spanish locale key: ${key}`).toHaveProperty(key);
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
