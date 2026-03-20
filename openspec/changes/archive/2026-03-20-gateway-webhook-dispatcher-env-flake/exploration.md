## Exploration: gateway-webhook-dispatcher-env-flake

### Current State
Archived verification for `gateway-dispatcher-parity` recorded one intermittent failure in `config::schema::tests::env_override_gateway_webhook_dispatcher`, then a clean exact rerun, and explicitly called out likely env-state interference rather than a confirmed dispatcher/runtime defect (`openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/verify-report.md:35`, `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/archive-report.md:37`). The two later follow-ups intentionally left this warning open (`openspec/changes/archive/2026-03-20-webhook-generated-session-isolation/exploration.md:42`, `openspec/changes/archive/2026-03-20-mcp-webhook-response-mapping/proposal.md:27`).

The production behavior under test is small and direct. Config env loading toggles `gateway.webhook_dispatcher_enabled` from `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` in `apply_gateway_env_overrides()` (`clients/agent-runtime/src/config/schema.rs:2799`). Gateway runtime-path selection also reads the same process env var directly in `webhook_dispatcher_enabled()` (`clients/agent-runtime/src/gateway/mod.rs:738`). There is no evidence yet that either production read path is logically wrong.

The test-isolation story is inconsistent:
- Config env-override tests serialize only within `config/schema.rs` via a module-local `ENV_OVERRIDE_TEST_LOCK` (`clients/agent-runtime/src/config/schema.rs:4751`).
- Gateway tests serialize only within `gateway/mod.rs` via a different module-local `GATEWAY_ENV_MUTEX` and mutate the same env var with `EnvVarGuard::set(...)` (`clients/agent-runtime/src/gateway/mod.rs:2126`, `clients/agent-runtime/src/gateway/mod.rs:3697`, `clients/agent-runtime/src/gateway/mod.rs:4389`).
- The flaky config test sets `CORVUS_GATEWAY_WEBHOOK_DISPATCHER=1` but does not restore or remove it after the assertion, unlike neighboring env-override tests (`clients/agent-runtime/src/config/schema.rs:5160`).

Because `clients/agent-runtime` defines both `src/lib.rs` and `src/main.rs`, the same module unit tests run in both test binaries, increasing opportunities for process-env mutation overlap when unrelated module-local locks do not coordinate (`clients/agent-runtime/src/lib.rs:42`, `clients/agent-runtime/src/main.rs:44`). Focused exact reruns for the flaky config test and a representative gateway dispatcher test both pass today, which supports the "intermittent shared-env race" theory but does not prove a deterministic reproducer.

### Affected Areas
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/verify-report.md` — original warning and failure evidence.
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/archive-report.md` — carry-forward archive note.
- `openspec/changes/gateway-webhook-dispatcher-env-flake/proposal.md` — already frames this as a minimal stabilization slice.
- `clients/agent-runtime/src/config/schema.rs` — flaky env-override test, local env lock, and override implementation.
- `clients/agent-runtime/src/gateway/mod.rs` — many tests mutate the same env var behind a different local lock; runtime path also reads the env var directly.
- `clients/agent-runtime/src/lib.rs` and `clients/agent-runtime/src/main.rs` — duplicate unit-test surfaces that amplify shared process-env contention.

### Approaches
1. **Proof-only cleanup in the single config test** — add explicit restore/remove behavior to `env_override_gateway_webhook_dispatcher` and maybe pre-clear the env before asserting defaults.
   - Pros: smallest diff; may remove leftover-state pollution from that exact test.
   - Cons: does not address cross-module races with gateway tests using a different lock on the same env var.
   - Effort: Low.

2. **Small shared env-test harness correction** — keep production code unchanged, but make `config/schema.rs` and gateway tests coordinate on one process-env lock for `CORVUS_GATEWAY_WEBHOOK_DISPATCHER`, plus restore env state consistently.
   - Pros: matches the most plausible root cause; still test-only; narrow surface; stabilizes both config and gateway test clusters.
   - Cons: touches more than one test module; needs a tiny shared helper or reused test-support seam.
   - Effort: Low/Medium.

3. **Production change in env resolution** — refactor config/gateway env reads or cache the flag differently.
   - Pros: only justified if a real runtime defect appears.
   - Cons: current evidence does not support it; broadens risk into config/runtime semantics for a test flake.
   - Effort: Medium.

### Recommendation
Use approach 2 unless a deterministic reproducer proves otherwise. The narrowest useful scope is not a broad config refactor and not a production behavior change; it is a small test-harness correction that serializes all mutations of `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` across the config and gateway test surfaces, plus explicit cleanup/restore in the flaky config test.

What should be in scope:
- Tight investigation and bounded reproduction around `config::schema::tests::env_override_gateway_webhook_dispatcher`.
- A shared or coordinated test-only env guard/lock for `CORVUS_GATEWAY_WEBHOOK_DISPATCHER`.
- Explicit env restore/remove behavior for the flaky test if still missing after the shared guard change.
- Focused validation that the config override test and at least one gateway dispatcher-path test remain stable together.

What should stay out of scope:
- Any change to dispatcher rollout semantics or webhook runtime behavior.
- Broader env-override framework redesign across the whole crate.
- MCP mapping, generated-session proof, `/whatsapp`, or verify-command plumbing.
- Spec behavior changes; this looks like evidence/harness correction, not new product behavior.

At this stage the best reading is: this follow-up is not proof-only in the narrowest sense, because a single extra assertion does not remove the race surface. It likely needs a small test-harness correction, but still no production change unless new evidence contradicts that.

### Risks
- The race may be rare enough that reproduction stays probabilistic, so the change will need to argue from structure as well as observed failure history.
- Over-correcting into a generic crate-wide env framework would violate the requested narrow scope.
- If a shared lock is introduced only for this variable, future env-sensitive tests may still have similar issues elsewhere.
- If a deterministic reproducer later shows production inconsistency between config loading and gateway flag reads, this exploration would need to be revisited before proposal/spec work.

### Ready for Proposal
Yes — with the scope framed as a small test-harness stabilization for `CORVUS_GATEWAY_WEBHOOK_DISPATCHER`, not as a dispatcher/runtime behavior change.
