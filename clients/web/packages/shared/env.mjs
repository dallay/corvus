/**
 * Shared environment helpers for web apps.
 *
 * Security defaults:
 * - Reject non-http(s) URLs
 * - Normalize host-only provider URLs to https
 * - Trim trailing slash for stable canonical values
 */

/** @typedef {'cloudflare' | 'vercel' | 'netlify' | 'local'} DeploymentProvider */

/**
 * @typedef {Object.<string, string | undefined>} EnvMap
 */

/**
 * @param {EnvMap} env
 * @returns {DeploymentProvider}
 */
export function detectDeploymentProvider(env = {}) {
  if (env.CF_PAGES || env.CF_PAGES_URL) {
    return "cloudflare";
  }
  if (env.VERCEL || env.VERCEL_URL) {
    return "vercel";
  }
  if (env.NETLIFY || env.URL || env.DEPLOY_URL) {
    return "netlify";
  }
  return "local";
}

/**
 * Read an env key with optional prefixes.
 * Order is deterministic and explicit to avoid hidden precedence bugs.
 *
 * @param {EnvMap} env
 * @param {string} key
 * @param {string[]} [prefixes]
 * @returns {string | undefined}
 */
export function getEnv(env, key, prefixes = ["", "PUBLIC_", "VITE_"]) {
  for (const prefix of prefixes) {
    const candidate = env[`${prefix}${key}`];
    if (candidate && candidate.trim().length > 0) {
      return candidate.trim();
    }
  }
  return undefined;
}

/**
 * @param {string} raw
 * @returns {string}
 */
function ensureHttpProtocol(raw) {
  const trimmed = raw.trim();
  if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
    return trimmed;
  }
  return `https://${trimmed}`;
}

/**
 * @param {string} raw
 * @returns {string}
 */
export function normalizeHttpUrl(raw) {
  const withProtocol = ensureHttpProtocol(raw);
  const parsed = new URL(withProtocol);

  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new Error(`Unsupported URL protocol for site config: ${parsed.protocol}`);
  }

  const normalizedPath = parsed.pathname === "/" ? "" : parsed.pathname.replace(/\/$/, "");
  const normalized = `${parsed.protocol}//${parsed.host}${normalizedPath}`;

  return normalized;
}

/**
 * @param {Object} config
 * @param {EnvMap} config.env
 * @param {string} config.primaryKey
 * @param {string} config.localDefault
 * @param {string} config.productionDefault
 * @param {string[]} [config.genericKeys]
 * @param {Object} [config.providerKeys]
 * @param {string} [config.providerKeys.cloudflare]
 * @param {string} [config.providerKeys.vercel]
 * @param {string} [config.providerKeys.netlify]
 * @param {boolean} [config.isProdLike]
 * @returns {string}
 */
export function resolveSiteUrl(config) {
  const {
    env,
    primaryKey,
    localDefault,
    productionDefault,
    genericKeys = ["SITE_URL"],
    providerKeys,
    isProdLike = false,
  } = config;

  const explicit = getEnv(env, primaryKey);
  if (explicit) {
    return normalizeHttpUrl(explicit);
  }

  const provider = detectDeploymentProvider(env);
  if (providerKeys) {
    const providerKey = providerKeys[provider];
    if (providerKey) {
      const providerValue = getEnv(env, providerKey, [""]);
      if (providerValue) {
        return normalizeHttpUrl(providerValue);
      }
    }
  }

  for (const key of genericKeys) {
    const value = getEnv(env, key);
    if (value) {
      return normalizeHttpUrl(value);
    }
  }

  return normalizeHttpUrl(isProdLike ? productionDefault : localDefault);
}

/**
 * @param {string} siteUrl
 * @param {number} fallbackPort
 * @returns {number}
 */
export function getPortFromUrl(siteUrl, fallbackPort) {
  const parsed = new URL(siteUrl);
  if (parsed.port) {
    const parsedPort = Number.parseInt(parsed.port, 10);
    if (Number.isFinite(parsedPort)) {
      return parsedPort;
    }
  }
  return fallbackPort;
}

export const PORTS = {
  MARKETING: 9988,
  PLUGINS: 9990,
  DOCS: 4321,
  CHAT: 4323,
};
