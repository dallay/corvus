# Proposal: Multi-Account Provider Pool

## Intent

Enable secure, automatic load balancing and failover across multiple provider accounts so the
runtime can spread traffic, reduce rate-limit impact, and improve reliability without manual
routing or operator intervention.

## Scope

### In Scope
- Add a first-class account pool model under reliability configuration for providers.
- Implement account selection in the reliability layer (e.g., round-robin/weighted) and apply
  credentials per request.
- Extend config parsing/validation to handle pooled credentials and ensure secrets are encrypted
  and redacted.
- Define whether admin HTTP config can read/update pool settings and update admin schema/contracts
  if enabled.
- It should be a reusable module that is independent of any specific system. Simply by adding the module, you should have access to all its functionality.

### Out of Scope
- Automatic account discovery or external credential fetchers.
- New provider types or model routing semantics beyond existing hints.
- Gateway webhook migration to canonical dispatcher behavior.

## Approach

Adopt an account pool in `ReliabilityConfig` and update `ReliableProvider` to select an account per
request, creating or caching per-account provider instances as needed. This makes pooling explicit
at the reliability layer and preserves existing routing semantics. Admin config exposure is
optional; if enabled, it will be limited to validated, redacted update paths.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/providers/reliable.rs` | Modified | Apply per-request account selection and credential injection. |
| `clients/agent-runtime/src/providers/mod.rs` | Modified | Build and cache providers per account; update factory wiring. |
| `clients/agent-runtime/src/config/schema.rs` | Modified | Add account pool config shape, validation, and secret handling. |
| `clients/agent-runtime/src/gateway/admin.rs` | Modified | Optional admin read/patch for pool config with redaction. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modified | Ensure pool config flows to runtime provider selection. |
| `clients/web/apps/dashboard/src/types/admin-config.ts` | Modified | Admin UI types/patch payloads if pool config is exposed. |
| `clients/agent-runtime/tests/admin_config_api_integration.rs` | Modified | Update admin API contract tests if pool config is exposed. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Credential rotation still ineffective if provider interfaces cannot accept per-request keys. | Medium | Introduce a credential-aware wrapper or per-account provider instances. |
| Admin API exposure increases secret handling complexity. | Medium | Redact responses, validate patches, and keep updates optional. |
| Pool selection could reduce per-account cache locality or increase overhead. | Low | Cache provider instances by account and reuse across calls. |

## Rollback Plan

Revert to single-account configuration by removing the account pool entries and using the existing
`provider.api_key` fields; disable admin pool patching if enabled. The runtime should ignore pool
config when absent and fall back to current reliability behavior.

## Dependencies

- Alignment on config schema changes and secret store handling for pooled credentials.
- Decision on admin HTTP support for pool read/update.

## Success Criteria

- [ ] Runtime can select among multiple accounts for a provider without manual routing hints.
- [ ] Rate-limit handling improves by distributing requests across accounts.
- [ ] Pooled credentials are encrypted at rest and redacted in logs/diagnostics.
- [ ] Admin config behavior (if enabled) is validated by integration tests.
