# Proposal: Unify onboarding and pairing flow across Corvus clients

## Intent

Corvus currently explains onboarding, pairing, activation, and connection differently across the
CLI/runtime, web dashboard, web chat, and composeApp mobile surfaces. This creates product drift,
confuses first-run users, and makes follow-up surface work hard to scope. This change defines one
canonical product-level onboarding story so every client can map to the same user outcomes while
still honoring the transport rules in `openspec/specs/client-surfaces/spec.md`.

## Scope

### In Scope
- Define the canonical product sequence from first-run discovery through ready-to-chat/session.
- Separate shared steps from surface-specific variants for CLI/runtime, web dashboard, web chat,
  and composeApp mobile/shared UX.
- Standardize user-facing terminology for pairing code, token acquisition, linking, and gateway
  connection.
- Define the normalized recovery/retry state taxonomy that every surface must expose.
- Clarify how this proposal relates to `openspec/specs/dashboard/spec.md` and
  `openspec/specs/client-surfaces/spec.md` so source-of-truth boundaries stay explicit.
- Identify affected modules/packages and create a clean split for follow-up implementation issues by
  surface.

### Out of Scope
- Changing the HTTP pairing protocol, bearer token storage model, or gateway auth semantics.
- Implementing gateway, dashboard, web chat, or composeApp code changes.
- Redesigning dashboard/admin UX outside the onboarding and connection story.
- Defining detailed API, UI copy, or platform bridge mechanics beyond product-level direction.

## Approach

Adopt a canonical product journey with transport-specific variants.

The proposal will direct later spec work to use this product sequence:

1. **Choose surface and intent** - user starts from operator setup, operator web management,
   end-user web chat, or end-user mobile chat.
2. **Confirm runtime availability** - the surface verifies the local Corvus runtime or bridge path
   is present and reachable.
3. **Establish trust with the runtime** - the surface completes a one-time trust step.
   - HTTP surfaces use **pairing**: get a pairing code, exchange it once, receive a bearer token.
   - Mobile uses **linking**: connect the app to the local CLI bridge/companion path without HTTP
     pairing terminology.
   - CLI/runtime is already trusted because it is the host surface.
4. **Connect the surface transport** - the surface validates its active connection path.
   - Dashboard and web chat validate gateway health and authenticated access.
   - Mobile validates bridge readiness and session-capable runtime access.
   - CLI validates local runtime readiness and optional dashboard activation guidance.
5. **Confirm ready state** - the user sees that Corvus is ready and what is available on that
   surface.
6. **Create or resume first session** - chat surfaces start or resume a UUID-based session;
   operator surfaces continue to management tasks instead of chat.

The terminology baseline is:

- **Pairing** = the HTTP gateway one-time code exchange that yields a bearer token.
- **Pairing code** = short-lived code shown by the runtime for HTTP clients only.
- **Bearer token** = persisted HTTP credential used by web dashboard and web chat after pairing.
- **Linking** = mobile trust establishment to the CLI bridge or companion path; not synonymous with
  HTTP pairing.
- **Connect to gateway** = validate an HTTP client can reach and authenticate to the gateway.
- **Connect to runtime** = umbrella product phrase for reaching a usable Corvus backend across any
  transport.

The proposal will direct later artifacts to treat recovery/retry as a shared product taxonomy with
surface-specific triggers:

- Runtime unavailable.
- Surface transport unavailable.
- Pairing code invalid or expired.
- Bearer token missing, invalid, or revoked.
- Gateway reachable but not paired/authenticated.
- Bridge linked but session start/resume unavailable.
- Session expired or no resumable session exists.
- Local environment unsupported for the chosen surface.

Source-of-truth boundary for later work:

- `openspec/specs/client-surfaces/spec.md` remains the transport/capability authority.
- `openspec/specs/dashboard/spec.md` remains the narrower operator dashboard activation spec.
- This change becomes the product-level source for cross-surface onboarding language, sequence,
  and recovery expectations, and later delta specs must reference those existing specs instead of
  redefining them independently.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/changes/2026-03-23-unify-onboarding-pairing-flow/proposal.md` | New | Product-level proposal for the unified onboarding story |
| `openspec/specs/client-surfaces/spec.md` | Referenced | Existing transport and capability source of truth to preserve |
| `openspec/specs/client-surfaces/surface-contracts/agent-runtime-cli.md` | Referenced | CLI/runtime role in the canonical sequence |
| `openspec/specs/client-surfaces/surface-contracts/web-dashboard.md` | Referenced | Operator web pairing/token path |
| `openspec/specs/client-surfaces/surface-contracts/web-chat.md` | Referenced | End-user web pairing/session path |
| `openspec/specs/client-surfaces/surface-contracts/composeapp-mobile.md` | Referenced | Mobile linking/bridge path |
| `openspec/specs/client-surfaces/surface-contracts/composeapp-shared.md` | Referenced | Shared KMP contract boundary for follow-up work |
| `openspec/specs/dashboard/spec.md` | Referenced | Existing narrower dashboard activation flow that must stay aligned |
| `openspec/specs/client-surfaces/migrations.md` | Referenced | Existing surface-splittable migration map for follow-up issues |
| `clients/agent-runtime/` | Future Modified | CLI/runtime onboarding copy and recovery messaging will align later |
| `clients/web/apps/dashboard/` | Future Modified | Dashboard token acquisition and activation wording will align later |
| `clients/web/apps/chat/` | Future Modified | Web chat onboarding and retry flow will be implemented later |
| `clients/composeApp/` | Future Modified | Mobile onboarding/linking flow will replace gateway-centric wording later |
| `modules/agent-core-kmp/` | Future Modified | Shared bridge/session contracts may need terminology alignment later |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Existing specs drift again after proposal approval | Medium | Make later specs reference this proposal and the capability matrix explicitly |
| Mobile linking is underspecified relative to current implementation reality | High | Keep proposal at product-outcome level and defer bridge mechanics to later spec/design work |
| HTTP-specific “pairing” language leaks into mobile UX | Medium | Reserve “pairing” for HTTP and use “linking” for mobile throughout follow-up artifacts |
| Inconsistent local entrypoint language persists | Medium | Later specs must choose one canonical user-facing local entrypoint and reuse it everywhere |
| Dashboard activation scope conflicts with cross-surface onboarding scope | Medium | Keep dashboard spec as a narrow operator slice and use this change only for cross-surface sequence and terminology |

## Rollback Plan

If this direction proves incorrect, revert by withdrawing this proposal and keeping onboarding source
of truth split between the existing dashboard activation spec and the per-surface client contracts.
Because this phase creates only a proposal artifact, rollback is limited to replacing or deleting
`openspec/changes/2026-03-23-unify-onboarding-pairing-flow/proposal.md` before any downstream specs
or implementation work lands.

## Dependencies

- `openspec/changes/2026-03-23-unify-onboarding-pairing-flow/exploration.md`
- `openspec/specs/client-surfaces/spec.md`
- `openspec/specs/client-surfaces/surface-contracts/agent-runtime-cli.md`
- `openspec/specs/client-surfaces/surface-contracts/web-dashboard.md`
- `openspec/specs/client-surfaces/surface-contracts/web-chat.md`
- `openspec/specs/client-surfaces/surface-contracts/composeapp-mobile.md`
- `openspec/specs/client-surfaces/surface-contracts/composeapp-shared.md`
- `openspec/specs/dashboard/spec.md`
- `openspec/specs/client-surfaces/migrations.md`

## Success Criteria

- [ ] The proposal defines one canonical onboarding sequence that applies across CLI/runtime, web
      dashboard, web chat, and composeApp mobile/shared UX.
- [ ] Shared steps and surface-specific variants are explicit enough to split follow-up work into
      per-surface issues.
- [ ] Pairing, pairing code, bearer token, linking, and gateway connection terminology are
      unambiguous at the product level.
- [ ] Recovery/retry expectations are normalized as a cross-surface taxonomy.
- [ ] The relationship to `openspec/specs/client-surfaces/spec.md` and
      `openspec/specs/dashboard/spec.md` is explicit so source-of-truth ownership stays clear.
