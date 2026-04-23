export type RookRoute =
  | "overview"
  | "accounts"
  | "pools"
  | "routes"
  | "health"
  | "usage"
  | "settings";

const DEFAULT_ROUTE: RookRoute = "overview";

export function normalizeHashRoute(hash: string | undefined | null): RookRoute {
  const value = (hash ?? "").trim().toLowerCase();

  if (value === "#/accounts" || value === "#accounts") {
    return "accounts";
  }

  if (value === "#/pools" || value === "#pools") {
    return "pools";
  }

  if (value === "#/routes" || value === "#routes") {
    return "routes";
  }

  if (value === "#/health" || value === "#health") {
    return "health";
  }

  if (value === "#/usage" || value === "#usage") {
    return "usage";
  }

  if (value === "#/settings" || value === "#settings") {
    return "settings";
  }

  if (value === "#/overview" || value === "#overview" || value === "#" || value === "") {
    return "overview";
  }

  return DEFAULT_ROUTE;
}

export function toHashRoute(route: RookRoute): string {
  return `#/${route}`;
}
