import { describe, it, expect } from "vitest";
import { resolveSiteUrl, getPortFromUrl, PORTS } from "../env.mjs";

const DEFAULT_DEV_URL = `http://localhost:${PORTS.DOCS}`;
const DEFAULT_PROD_URL = "https://docs.profiletailors.com";

describe("resolveSiteUrl and getPortFromUrl matrix", () => {
  it("resolveSiteUrl_prefers_DOCS_URL_over_providers_and_falls_back_to_SITE_URL", () => {
    // Explicit DOCS_URL should win
    const explicit = resolveSiteUrl({
      env: { DOCS_URL: "http://explicit.example/path" },
      primaryKey: "DOCS_URL",
      localDefault: DEFAULT_DEV_URL,
      productionDefault: DEFAULT_PROD_URL,
      genericKeys: ["SITE_URL"],
      providerKeys: { cloudflare: "CF_PAGES_URL", vercel: "VERCEL_URL", netlify: "URL" },
      isProdLike: false,
    });
    expect(explicit).toBe("http://explicit.example/path");

    // Cloudflare provider should be selected when DOCS_URL missing
    const cloudflare = resolveSiteUrl({
      env: { CF_PAGES_URL: "docs.cloudflare.test/sub" },
      primaryKey: "DOCS_URL",
      localDefault: DEFAULT_DEV_URL,
      productionDefault: DEFAULT_PROD_URL,
      genericKeys: ["SITE_URL"],
      providerKeys: { cloudflare: "CF_PAGES_URL", vercel: "VERCEL_URL", netlify: "URL" },
      isProdLike: false,
    });
    expect(cloudflare).toBe("https://docs.cloudflare.test/sub");

    // Vercel provider
    const vercel = resolveSiteUrl({
      env: { VERCEL_URL: "my-vercel.app/docs" },
      primaryKey: "DOCS_URL",
      localDefault: DEFAULT_DEV_URL,
      productionDefault: DEFAULT_PROD_URL,
      genericKeys: ["SITE_URL"],
      providerKeys: { cloudflare: "CF_PAGES_URL", vercel: "VERCEL_URL", netlify: "URL" },
      isProdLike: false,
    });
    expect(vercel).toBe("https://my-vercel.app/docs");

    // Netlify provider (URL)
    const netlify = resolveSiteUrl({
      env: { URL: "https://netlify.example/base/" },
      primaryKey: "DOCS_URL",
      localDefault: DEFAULT_DEV_URL,
      productionDefault: DEFAULT_PROD_URL,
      genericKeys: ["SITE_URL"],
      providerKeys: { cloudflare: "CF_PAGES_URL", vercel: "VERCEL_URL", netlify: "URL" },
      isProdLike: false,
    });
    expect(netlify).toBe("https://netlify.example/base");

    // Generic SITE_URL fallback
    const generic = resolveSiteUrl({
      env: { SITE_URL: "http://fallback.test:9001/fpath" },
      primaryKey: "DOCS_URL",
      localDefault: DEFAULT_DEV_URL,
      productionDefault: DEFAULT_PROD_URL,
      genericKeys: ["SITE_URL"],
      providerKeys: { cloudflare: "CF_PAGES_URL", vercel: "VERCEL_URL", netlify: "URL" },
      isProdLike: false,
    });
    expect(generic).toBe("http://fallback.test:9001/fpath");

    // No envs -> dev fallback
    const fallbackDev = resolveSiteUrl({
      env: {},
      primaryKey: "DOCS_URL",
      localDefault: DEFAULT_DEV_URL,
      productionDefault: DEFAULT_PROD_URL,
      genericKeys: ["SITE_URL"],
      providerKeys: { cloudflare: "CF_PAGES_URL", vercel: "VERCEL_URL", netlify: "URL" },
      isProdLike: false,
    });
    expect(fallbackDev).toBe(DEFAULT_DEV_URL);

    // No envs -> prod fallback when isProdLike
    const fallbackProd = resolveSiteUrl({
      env: {},
      primaryKey: "DOCS_URL",
      localDefault: DEFAULT_DEV_URL,
      productionDefault: DEFAULT_PROD_URL,
      genericKeys: ["SITE_URL"],
      providerKeys: { cloudflare: "CF_PAGES_URL", vercel: "VERCEL_URL", netlify: "URL" },
      isProdLike: true,
    });
    expect(fallbackProd).toBe(DEFAULT_PROD_URL);
  });

  it("getPortFromUrl_derives_port_from_docsUrl", () => {
    // URL with explicit port
    const withPort = "http://example.com:8080/path";
    expect(getPortFromUrl(withPort, PORTS.DOCS)).toBe(8080);

    // URL without port -> fallback
    const withoutPort = "https://example.com/path";
    expect(getPortFromUrl(withoutPort, PORTS.DOCS)).toBe(PORTS.DOCS);

    // Derived from resolveSiteUrl result
    const docsUrl = resolveSiteUrl({
      env: { DOCS_URL: "http://localhost:12345/docs" },
      primaryKey: "DOCS_URL",
      localDefault: DEFAULT_DEV_URL,
      productionDefault: DEFAULT_PROD_URL,
      genericKeys: ["SITE_URL"],
      providerKeys: { cloudflare: "CF_PAGES_URL", vercel: "VERCEL_URL", netlify: "URL" },
      isProdLike: false,
    });
    expect(getPortFromUrl(docsUrl, PORTS.DOCS)).toBe(12345);
  });
});
