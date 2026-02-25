---
name: plugin-release
description:
  Use this skill when asked to release or publish Corvus runtime plugins.
  Covers tag-driven plugin workflow, immutable artifact publishing, signing,
  catalog update, and Cloudflare Pages deployment checks.
---

# Plugin Release Manager

This skill is specific to plugin releases (not full product releases).

## Scope

Use this for plugins published through `.github/workflows/publish-plugins.yml`,
with tags in this exact form:

- `plugin/<plugin-id>/v<semver>`
- Example: `plugin/memory.surreal.graphs/v0.2.0`

Runtime note:

- End users do **not** need local `cosign` installed to verify plugins.
- Runtime verification is native Sigstore verification inside `corvus`.

## Preconditions

Before tagging a release, verify:

1. Plugin metadata exists in the plugin `Cargo.toml`:
   - `package.metadata.corvus.plugin_id`
   - `runtime_api`, `capabilities`, `memory_entrypoint`, `health_entrypoint`
2. Plugin version in `Cargo.toml` matches tag semver.
3. Plugin builds for `wasm32-wasip1`.
4. Git working tree is clean (no staged or unstaged changes).
5. Catalog deployment secrets are configured in GitHub:
   - `CLOUDFLARE_API_TOKEN`
   - `CLOUDFLARE_ACCOUNT_ID`
   - `CLOUDFLARE_PAGES_PROJECT_NAME`

## Release Flow

### 1) Verify local state

```bash
git status
git log --oneline -10
```

Stop if `git status` reports any uncommitted or staged changes (see Preconditions).

### 2) Verify plugin manifest metadata

```bash
cat clients/agent-runtime/plugins/<plugin-dir>/Cargo.toml
```

Confirm:

- `version = "X.Y.Z"`
- `plugin_id = "<plugin-id>"`

### 3) Build plugin artifact locally

```bash
cargo build \
  --manifest-path clients/agent-runtime/plugins/<plugin-dir>/Cargo.toml \
  --target wasm32-wasip1 \
  --release
```

### 4) Create release tag

```bash
git tag -a plugin/<plugin-id>/vX.Y.Z -m "Release <plugin-id> vX.Y.Z"
git push origin plugin/<plugin-id>/vX.Y.Z
```

This triggers `publish-plugins.yml`.

#### Recovery (if tag pushed with wrong version)

```bash
git tag -d plugin/<plugin-id>/vX.Y.Z
git push origin --delete plugin/<plugin-id>/vX.Y.Z
```

Then:

- if workflow already started for the wrong tag, inspect that GitHub Actions run first (`publish-plugins.yml`)
- remove any wrong immutable artifact path from Cloudflare Pages (`artifacts/<plugin-id>/<wrong-version>/`) via Pages dashboard or Pages API
- revert any `catalog.json` / `plugin-manifest.json` upsert that points to the wrong version before re-tagging
- fix version in `clients/agent-runtime/plugins/<plugin-dir>/Cargo.toml`
- commit the correction
- re-run publish by creating the correct `plugin/<plugin-id>/vX.Y.Z` tag
- if needed, re-trigger `.github/workflows/publish-plugins.yml`

### 5) Monitor workflow and verify outputs

Validate workflow artifacts and deployment:

- immutable path published:
  - `artifacts/<plugin-id>/<version>/<plugin-id>.wasm` (`<version>` is bare semver from `Cargo.toml`, not tag-form `v<semver>`)
  - Example: `artifacts/memory.surreal.graphs/0.2.0/memory.surreal.graphs.wasm` (not `.../v0.2.0/...`)
- signature produced:
  - `artifacts/<plugin-id>/<version>/<plugin-id>.wasm.sig`
- certificate produced for keyless signing:
  - `artifacts/<plugin-id>/<version>/<plugin-id>.wasm.pem`
- `plugin-manifest.json` updated for this version
- `catalog.json` upserted without breaking other plugin entries

## Signing Rules

Publishing supports two modes:

1. **Key-based cosign** (if `COSIGN_PRIVATE_KEY` secret exists)
2. **Keyless OIDC cosign** (fallback/default)

Notes:

- Key-based signing does not always emit a `.pem` certificate file.
- Keyless OIDC signing emits certificate metadata suitable for identity checks.
- The certificate issuer for GitHub Actions keyless signing is
  `https://token.actions.githubusercontent.com` (OIDC issuer identity),
  not your catalog domain.

## Cloudflare Pages Rules

For static hosting compatibility:

- Use `_headers` patterns compatible with Cloudflare (single splat wildcard behavior).
- Ensure artifact routes include:
  - immutable cache headers
  - correct content-type
  - CORS for runtime installs

## Common Failure Cases

1. Tag/version mismatch (see Recovery section above):
   - Tag says `v0.2.0` but `Cargo.toml` still `0.1.0`.
2. Missing `package.metadata.corvus.plugin_id`.
3. Missing Cloudflare secrets.
4. Catalog URL/base path misconfiguration causing broken artifact URLs.
5. Signature policy mismatch for official source identity.

## Post-release Checklist

- [ ] Workflow succeeded in GitHub Actions
- [ ] Artifact is reachable at immutable URL
- [ ] Catalog entry points to new immutable URL
- [ ] Runtime can install plugin via `corvus plugins install <plugin-id>`; run `corvus plugins install <plugin-id>` and require exit code 0.
- [ ] Runtime verification passes for the installed plugin; run `corvus plugins verify <plugin-id>` (or equivalent runtime verification command), require exit code 0, and assert signature verification succeeded in the output.
