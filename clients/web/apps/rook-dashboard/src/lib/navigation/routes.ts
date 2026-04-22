export type RookRoute = "overview" | "accounts";

const DEFAULT_ROUTE: RookRoute = "overview";

export function normalizeHashRoute(hash: string | undefined | null): RookRoute {
  const value = (hash ?? "").trim().toLowerCase();

  if (value === "#/accounts" || value === "#accounts") {
    return "accounts";
  }

  if (value === "#/overview" || value === "#overview" || value === "#" || value === "") {
    return "overview";
  }

  return DEFAULT_ROUTE;
}

export function toHashRoute(route: RookRoute): string {
  return `#/${route}`;
}
