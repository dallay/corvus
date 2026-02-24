interface R2Object {
  body: ReadableStream<Uint8Array> | null;
  size: number;
  httpEtag?: string;
  httpMetadata?: {
    contentType?: string;
  };
}

interface R2BucketLike {
  get(key: string): Promise<R2Object | null>;
}

interface Env {
  PLUGINS_BUCKET: R2BucketLike;
  CATALOG_OBJECT_KEY?: string;
  REVOCATIONS_OBJECT_KEY?: string;
}

const DEFAULT_CATALOG_KEY = "catalog/catalog.json";
const DEFAULT_REVOCATIONS_KEY = "catalog/revocations.json";
const ARTIFACTS_PREFIX = "/artifacts/";

const RESPONSE_HEADERS = {
  nosniff: "X-Content-Type-Options",
  cacheControl: "Cache-Control",
  contentType: "Content-Type",
  contentLength: "Content-Length",
  etag: "ETag",
  allowOrigin: "Access-Control-Allow-Origin",
  allowMethods: "Access-Control-Allow-Methods",
  allowHeaders: "Access-Control-Allow-Headers",
} as const;

const CACHE_POLICIES = {
  catalog: "public, max-age=300, stale-while-revalidate=60",
  revocations: "no-store, max-age=0",
  artifact: "public, max-age=31536000, immutable",
} as const;

const METHOD_NOT_ALLOWED_HEADERS = {
  Allow: "GET, HEAD, OPTIONS",
};

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    if (request.method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: corsHeaders(),
      });
    }

    if (!isSupportedMethod(request.method)) {
      return new Response("Method Not Allowed", {
        status: 405,
        headers: METHOD_NOT_ALLOWED_HEADERS,
      });
    }

    const url = new URL(request.url);
    const pathname = normalizePathname(url.pathname);

    if (pathname === "/catalog.json") {
      return serveObject(
        request,
        env,
        env.CATALOG_OBJECT_KEY || DEFAULT_CATALOG_KEY,
        "application/json; charset=utf-8",
        CACHE_POLICIES.catalog,
        false
      );
    }

    if (pathname === "/revocations.json") {
      return serveObject(
        request,
        env,
        env.REVOCATIONS_OBJECT_KEY || DEFAULT_REVOCATIONS_KEY,
        "application/json; charset=utf-8",
        CACHE_POLICIES.revocations,
        false
      );
    }

    if (pathname.startsWith(ARTIFACTS_PREFIX)) {
      const key = safeArtifactKey(pathname);
      if (!key) {
        return jsonError(400, "Invalid artifact path");
      }

      return serveObject(
        request,
        env,
        key,
        guessArtifactContentType(key),
        CACHE_POLICIES.artifact,
        true
      );
    }

    return jsonError(404, "Not found");
  },
};

function isSupportedMethod(method: string): boolean {
  return method === "GET" || method === "HEAD";
}

function normalizePathname(pathname: string): string {
  if (!pathname || pathname === "/") {
    return "/";
  }

  const decoded = decodeURIComponent(pathname);
  return decoded.replace(/\/{2,}/g, "/");
}

function safeArtifactKey(pathname: string): string | null {
  if (!pathname.startsWith(ARTIFACTS_PREFIX)) {
    return null;
  }

  if (pathname.includes("..") || pathname.includes("\\")) {
    return null;
  }

  const candidate = pathname.slice(1);
  if (!candidate) {
    return null;
  }

  const valid = /^[a-zA-Z0-9._\-/]+$/.test(candidate);
  if (!valid) {
    return null;
  }

  return candidate;
}

async function serveObject(
  request: Request,
  env: Env,
  key: string,
  fallbackContentType: string,
  cacheControl: string,
  withCors: boolean
): Promise<Response> {
  const object = await env.PLUGINS_BUCKET.get(key);
  if (!object) {
    return jsonError(404, "Not found");
  }

  const headers = new Headers();
  headers.set(RESPONSE_HEADERS.nosniff, "nosniff");
  headers.set(RESPONSE_HEADERS.cacheControl, cacheControl);
  headers.set(
    RESPONSE_HEADERS.contentType,
    object.httpMetadata?.contentType || fallbackContentType
  );
  headers.set(RESPONSE_HEADERS.contentLength, String(object.size));

  if (object.httpEtag) {
    headers.set(RESPONSE_HEADERS.etag, object.httpEtag);
  }

  if (withCors) {
    const cors = corsHeaders();
    cors.forEach((value, name) => {
      headers.set(name, value);
    });
  }

  const ifNoneMatch = request.headers.get("If-None-Match");
  if (ifNoneMatch && object.httpEtag && ifNoneMatch === object.httpEtag) {
    return new Response(null, {
      status: 304,
      headers,
    });
  }

  if (request.method === "HEAD") {
    return new Response(null, {
      status: 200,
      headers,
    });
  }

  return new Response(object.body, {
    status: 200,
    headers,
  });
}

function guessArtifactContentType(key: string): string {
  if (key.endsWith(".wasm")) {
    return "application/wasm";
  }
  if (key.endsWith(".json")) {
    return "application/json; charset=utf-8";
  }
  if (key.endsWith(".pem")) {
    return "application/x-pem-file";
  }
  if (key.endsWith(".sig")) {
    return "application/octet-stream";
  }
  return "application/octet-stream";
}

function corsHeaders(): Headers {
  const headers = new Headers();
  headers.set(RESPONSE_HEADERS.allowOrigin, "*");
  headers.set(RESPONSE_HEADERS.allowMethods, "GET, HEAD, OPTIONS");
  headers.set(RESPONSE_HEADERS.allowHeaders, "Content-Type, Authorization, If-None-Match");
  return headers;
}

function jsonError(status: number, message: string): Response {
  return new Response(JSON.stringify({ error: message }), {
    status,
    headers: {
      [RESPONSE_HEADERS.contentType]: "application/json; charset=utf-8",
      [RESPONSE_HEADERS.nosniff]: "nosniff",
      [RESPONSE_HEADERS.cacheControl]: "no-store, max-age=0",
    },
  });
}
