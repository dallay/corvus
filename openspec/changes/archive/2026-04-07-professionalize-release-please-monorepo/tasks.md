# Tasks: Professionalize Release Please Monorepo

## Phase 1: Release Scope Foundation

- [x] 1.1 Update `release-please-config.json` to replace broad
  `clients/agent-runtime/npm/**/package.json` and `clients/web/**/package.json` fan-out with
  explicit shipped files only: root version files, Cargo/Gradle targets,
  `clients/agent-runtime/npm/corvus/package.json`, and platform packages
  `corvus-{darwin-x64,darwin-arm64,linux-x64,linux-arm64,windows-x64}/package.json`.
- [x] 1.2 Remove `bootstrap-sha` from `release-please-config.json`; keep
  `.release-please-manifest.json` on the verified baseline and capture any missing-tag /
  missing-release recovery as a manual operator step rather than workflow automation.
- [x] 1.3 Update `clients/agent-runtime/npm/corvus/package.json` to drop
  `@dallay/corvus-windows-arm64` from `optionalDependencies` so npm metadata matches supported
  platforms in `clients/agent-runtime/npm/corvus/bin/corvus.js`.
- [x] 1.4 Leave `clients/agent-runtime/npm/corvus-cli/package.json` and
  `clients/agent-runtime/npm/corvus-windows-arm64/package.json` outside stable version fan-out; if
  touched for clarity, mark them internal / unsupported rather than shipped artifacts.

## Phase 2: Workflow Contract and Observability

- [x] 2.1 Update `.github/workflows/release-please.yml` to expose release-please outputs and write a
  `$GITHUB_STEP_SUMMARY` with manifest version, PR created/updated state, and tag/release outputs
  for baseline diagnosis.
- [x] 2.2 Update `.github/workflows/publish-release.yml` and
  `.github/workflows/publish-snapshot.yml` comments/inputs so stable publish is tag-only and
  snapshot publish is explicitly outside GitHub Release and release-note ownership.
- [x] 2.3 Update `.github/workflows/_publish.yml` so stable version checks and summaries only cover
  shipped artifacts, report pass/fail by surface, and keep GitHub Release creation/notes ownership
  in publish.
- [x] 2.4 Ensure `_publish.yml` npm publishing policy and summaries do not imply stable publication
  for `corvus-cli` or Windows ARM64.

## Phase 3: Documentation and Canonical Notes

- [x] 3.1 Update `clients/web/apps/docs/src/content/docs/guides/release.md` with the canonical
  PR/tag flow, publish-owned GitHub Releases, private web-package exclusion, `corvus-cli` exclusion,
  Windows ARM64 unsupported status, and manual baseline recovery steps.
- [x] 3.2 Update `.github/workflows/README.md` to match the same stable-vs-snapshot contract and
  workflow ownership boundaries.
- [x] 3.3 Replace stale `CHANGELOG.md` ledger content with a short pointer to GitHub Releases as the
  canonical stable release-notes source.

## Phase 4: Verification

- [x] 4.1 Verify the release-please file set so only shipped artifacts would change in a release PR;
  web manifests, `clients/agent-runtime/npm/corvus-cli/package.json`, and
  `clients/agent-runtime/npm/corvus-windows-arm64/package.json` must remain outside stable version
  churn.
- [x] 4.2 Validate touched `.github/workflows/*.yml`, `release-please-config.json`, and
  `package.json` files for syntax, then inspect workflow summaries to confirm the new diagnostics
  render the expected release-state fields.
- [x] 4.3 Rehearse a safe non-production validation path (branch/manual run or documented dry-run)
  and confirm the runbook instructs operator-driven baseline/tag recovery instead of mutating live
  tags or releases in code.

## Phase 5: Backfilled Release Blocker Follow-up

- [x] 5.1 Fix `_publish.yml` to invoke Maven Central publishing from `gradle/build-logic`, where
  Vanniktech publish tasks actually exist for this repository layout.
- [x] 5.2 Add the `cerebro` dependency version to `clients/agent-runtime/Cargo.toml` and teach
  `release-please-config.json` to keep that dependency pin aligned with future repo-wide bumps.
- [x] 5.3 Refresh `clients/agent-runtime/Cargo.lock` and `clients/cerebro/Cargo.lock` so release
  workflows using `--locked` no longer fail on stale lockfiles.
- [x] 5.4 Extend lightweight release-contract regression coverage for the Gradle publish invocation
  and Rust dependency-version alignment without requiring a full build.
- [x] 5.5 Update minimal operator documentation that still referenced the invalid root
  `publishToMavenCentral` invocation.
