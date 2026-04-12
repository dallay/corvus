import * as axe from "axe-core";
import { expect } from "vitest";

type AxeOptions = Parameters<typeof axe.run>[1];

function formatViolations(violations: axe.Result[]): string {
  return violations
    .map((violation) => {
      const nodes = violation.nodes
        .map((node) => `${node.target.join(" ")} :: ${node.failureSummary ?? "no summary"}`)
        .join("\n");
      return `${violation.id}: ${violation.help}\n${nodes}`;
    })
    .join("\n\n");
}

export async function expectNoAxeViolations(
  element: Element,
  options: AxeOptions = {}
): Promise<void> {
  const context = element.isConnected ? element : (element.ownerDocument?.body ?? element);

  const results = await axe.run(context, {
    rules: {
      "color-contrast": { enabled: false },
      ...(options.rules ?? {}),
    },
    ...options,
  });

  expect(results.violations, formatViolations(results.violations)).toHaveLength(0);
}
