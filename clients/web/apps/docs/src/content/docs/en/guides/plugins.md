---
title: Runtime Plugins
---

This guide is the team playbook for adding a new official runtime plugin in Corvus.

It covers:

- Plugin contract and repository layout
- Secure build and publish workflow
- Catalog/revocation integration
- Wizard and runtime integration
- Validation and rollout checklist

## 1. Plugin Model (Current State)

Corvus plugins are distributed as signed WASM artifacts plus metadata.

- Artifact: `<plugin-id>.wasm`
- Metadata: `plugin-manifest.json`, `catalog.json`, `revocations.json`
- Distribution: OCI (optional in workflow), plus uploaded bundle artifacts
- Runtime policy: allowlisted publishers, digest verification, revocation checks, lockfile pinning

Key runtime code:

- `src/plugins/mod.rs`
- `src/config/schema.rs`

## 2. Contract and Layout

### 2.1 WIT Contract

Use the shared WIT contract:

- `plugins/wit/corvus-plugin.wit`

This defines exported interfaces for memory, health, and plugin capabilities.

### 2.2 Plugin Source Layout

Create a new directory under:

- `plugins/<your-plugin-folder>/`

Minimum expected files:

- `Cargo.toml` with `crate-type = ["cdylib"]`
- `src/lib.rs`

Reference plugin:

- `plugins/memory-surreal-graphs/`

## 3. Security Requirements (Non-Negotiable)

All new official plugins must satisfy:

1. HTTPS catalog/revocation sources in configuration.
2. Trusted publisher allowlist (`corvus-official` by default).
3. Digest pinning in lockfile after installation.
4. Revocation support and enforced revocation checks.
5. No runtime startup hard-fail of the entire agent on plugin install/load issues; core fallback
   must remain available when designed.

Relevant configuration defaults:

- Catalog: `https://plugins.corvus.profiletailors.com/catalog.json`
- Revocations: `https://plugins.corvus.profiletailors.com/revocations.json`

See:

- `src/config/schema.rs`

## 4. Add a New Plugin (Implementation Steps)

### 4.1 Create the plugin crate

1. Add `plugins/<new-plugin>/Cargo.toml`.
2. Set:
  - `edition = "2021"`
  - `crate-type = ["cdylib"]`
3. Add minimal exported entrypoint(s) aligned with your WIT contract usage.

### 4.2 Build locally

From repo root:

```bash
cargo build \
  --manifest-path clients/agent-runtime/plugins/<new-plugin>/Cargo.toml \
  --target wasm32-wasip1 \
  --release
```

### 4.3 Runtime registration/integration

If the plugin is installable via dedicated flow (like Surreal Graphs), add/update:

- Plugin constant and install helpers in `src/plugins/mod.rs`
- Any onboarding/wizard hooks in `src/onboard/wizard.rs`

If onboarding should expose it:

1. Add option text in wizard memory/backend selection.
2. Ensure plugin installation check runs before collecting backend-specific options that depend on
   it.
3. Ensure failure path is explicit and safe.

## 5. Publish Workflow

Workflow file:

- `.github/workflows/publish-plugins.yml`

Current workflow behavior:

1. Trigger automatically on plugin release tags: `plugin/<plugin-id>/v<semver>`.
2. Resolve plugin directory dynamically from `package.metadata.corvus.plugin_id` in each
   plugin `Cargo.toml`.
3. Build WASM plugin artifact for `wasm32-wasip1`.
4. Assemble immutable bundle metadata and artifacts:
   - `artifacts/<plugin-id>/<version>/<plugin-id>.wasm`
   - `artifacts/<plugin-id>/<version>/plugin-manifest.json`
   - root `catalog.json` (upsert plugin entry, keep others)
   - root `revocations.json` (preserve list, refresh `updated_at`)
5. Sign artifact with cosign keyless OIDC identity.
6. Verify signature in CI.
7. Optionally push artifact bundle to OCI (`oci_repository`).
8. Build plugins catalog app and deploy to Cloudflare Pages (enabled by default for release tags).
9. Upload build + bundle artifacts as workflow artifacts for traceability.

:::important
To onboard a new plugin into automated releases:

1. Create it under `clients/agent-runtime/plugins/<plugin-folder>/`.
2. Add `package.metadata.corvus.plugin_id` to its `Cargo.toml`.
3. Set plugin limits/capabilities in `package.metadata.corvus`.
4. Create a release tag: `plugin/<plugin-id>/v<version>`.

No workflow code changes are required for new plugins when metadata is present.
:::

Release example:

```bash
git tag plugin/memory.surreal.graphs/v0.1.0
git push origin plugin/memory.surreal.graphs/v0.1.0
```

Cloudflare deployment configuration expected by the workflow:

- Secret: `CLOUDFLARE_API_TOKEN`
- Secret: `CLOUDFLARE_ACCOUNT_ID`
- Repository variable: `CLOUDFLARE_PAGES_PROJECT_NAME`

## 6. Operator Commands (Runtime)

Plugin lifecycle commands:

```bash
corvus plugins list
corvus plugins install <plugin-id> [--version <semver>] [--source <source-name>]
corvus plugins verify [--id <plugin-id>]
corvus plugins pin <plugin-id> [--version <semver>]
corvus plugins remove <plugin-id>
corvus plugins revocations sync
```

## 7. Validation Checklist for New Plugins

Before merge:

- [ ] Build plugin for `wasm32-wasip1`.
- [ ] Validate manifest/catalog/revocations fields and digest integrity.
- [ ] Verify install + verify commands work with local or test catalog.
- [ ] Confirm revocation behavior:
  - revoked plugin is blocked as expected
- [ ] Confirm onboarding behavior if wizard-integrated:
  - plugin-required path is explicit
  - failure path is user-readable and safe
- [ ] Confirm lockfile reproducibility:
  - `~/.corvus/plugins.lock` contains expected ID/version/digest/source

## 8. Rollout Strategy

Recommended production rollout:

1. Publish plugin artifact and metadata.
2. Enable for internal canary users first.
3. Monitor install/verify errors and startup behavior.
4. Expand rollout after clean telemetry.
5. Keep revocation list operational and tested.

## 9. Troubleshooting

### Install fails with trust/publisher errors

- Check `[plugins].allow_publishers` in config.
- Check manifest publisher value.

### Install fails with digest mismatch

- Rebuild/republish artifact and regenerate manifest digest.
- Verify source catalog points to correct artifact digest.

### Revocation sync issues

- Run `corvus plugins revocations sync`.
- Check configured `[plugins.revocation].source_urls`.
- If enforcement is enabled, broken revocation sources can block plugin operations by design.

### Migration from old plugin host

On config load, Corvus migrates old `plugins.corvus.ai` host references to
`plugins.corvus.profiletailors.com` for both catalog and revocation source URLs.
