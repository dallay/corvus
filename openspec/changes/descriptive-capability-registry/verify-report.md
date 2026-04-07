## Verification Report

**Change**: descriptive-capability-registry
**Version**: M2 delta against `capability-architecture`

---

### Completeness
| Metric | Value |
|--------|-------|
| Tasks total | 16 |
| Tasks complete | 16 |
| Tasks incomplete | 0 |

All tasks in `openspec/changes/descriptive-capability-registry/tasks.md` are marked complete.

---

### Build & Tests Execution

**Build**: ➖ Not configured as a separate verify build command in `openspec/config.yaml`

**Targeted tests**:
- `cargo test --manifest-path "clients/agent-runtime/Cargo.toml" --lib capabilities -- --nocapture` ✅ 56 passed / 0 failed
- `cargo test --manifest-path "clients/agent-runtime/Cargo.toml" --lib bootstrap_registry_matches_final_active_tool_set_after_profile_filtering -- --nocapture` ✅ 1 passed / 0 failed
- `cargo test --manifest-path "clients/agent-runtime/Cargo.toml" --lib registry_ids_preserve_existing_profile_and_mcp_prefix_classification -- --nocapture` ✅ 1 passed / 0 failed

**Full tests**:
- `cargo test --manifest-path "clients/agent-runtime/Cargo.toml"` ✅ Passed
- Evidence from final output: unit/integration/doc tests completed successfully; final visible tail shows last suite pass and doc-tests pass.

**Formatting / lint**:
- `cargo fmt --manifest-path "clients/agent-runtime/Cargo.toml" --all -- --check` ✅ Passed
- `cargo clippy --manifest-path "clients/agent-runtime/Cargo.toml" --all-targets -- -D warnings` ✅ Passed

**Coverage**: ➖ Not configured

---

### Spec Compliance Matrix

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| M2 Tool-Family Descriptive Registry | Registry coexists with legacy execution authority | `src/bootstrap/mod.rs > bootstrap_context_builds_core_components`; full `cargo test` agent tool-loop suite | ✅ COMPLIANT |
| M2 Tool-Family Descriptive Registry | Registry is limited to tool-family capabilities in M2 | Type-level enforcement in `src/capabilities/descriptor.rs`; no dedicated runtime test | ⚠️ PARTIAL |
| M2 Tool-Family Descriptive Registry | Registry does not alter provider or channel tool payload generation | `src/agent/tests.rs > native_dispatcher_sends_tool_specs`; `src/channels/mod.rs > prompt_contains_channel_capabilities` | ⚠️ PARTIAL |
| M2 Tool-Family Descriptor Minimum Contract | Native tool descriptor preserves current tool identity | `src/capabilities/tool_registration.rs > builds_native_tool_descriptor_with_defaults` | ✅ COMPLIANT |
| M2 Tool-Family Descriptor Minimum Contract | MCP-derived tool-layer descriptor preserves canonical identity | `src/capabilities/tool_registration.rs > builds_mcp_tool_descriptor_with_canonical_metadata`, `builds_mcp_resource_descriptor_with_uri_and_mime_type`, `builds_mcp_prompt_descriptor_with_arguments` | ✅ COMPLIANT |
| M2 Tool-Family Descriptor Minimum Contract | Descriptor completeness is required even with empty dependencies | `src/capabilities/registry.rs > rejects_missing_required_fields`; `src/capabilities/tool_registration.rs > builds_native_tool_descriptor_with_defaults` | ✅ COMPLIANT |
| M2 Registration Timing and Active-Scope Finalization | Registry is finalized after profile filtering | `src/bootstrap/mod.rs > bootstrap_registry_matches_final_active_tool_set_after_profile_filtering` | ✅ COMPLIANT |
| M2 Registration Timing and Active-Scope Finalization | Inactive capabilities are not registered as active M2 descriptors | `src/bootstrap/mod.rs > bootstrap_registry_matches_final_active_tool_set_after_profile_filtering` | ✅ COMPLIANT |
| M2 Deterministic Validation and Collision Handling | Duplicate namespaced identities are rejected deterministically | `src/capabilities/registry.rs > rejects_duplicate_ids` | ✅ COMPLIANT |
| M2 Deterministic Validation and Collision Handling | Invalid descriptor completeness is rejected deterministically | `src/capabilities/registry.rs > rejects_missing_required_fields` | ✅ COMPLIANT |
| M2 Deterministic Validation and Collision Handling | Cross-kind MCP identities remain valid when canonical ids differ | `src/tools/mcp/mod.rs > discover_capabilities_tool_and_resource_same_name_coexist`; `discover_capabilities_tool_and_prompt_same_name_coexist` | ✅ COMPLIANT |
| M2 Deterministic Validation and Collision Handling | Native and MCP naming conflicts are handled explicitly | `src/capabilities/registry.rs > explicit_collision_policy_is_duplicate_error_for_same_visible_id`; `tests/mcp_native_regression.rs > native_tool_registry_is_stable_with_mcp_enabled_or_disabled` | ⚠️ PARTIAL |
| MCP Tool-Layer Mapping Under the Shared Descriptor Contract | MCP tool-layer capabilities register under the shared contract | `src/capabilities/tool_registration.rs > builds_mcp_tool_descriptor_with_canonical_metadata`, `builds_mcp_resource_descriptor_with_uri_and_mime_type`, `builds_mcp_prompt_descriptor_with_arguments` | ✅ COMPLIANT |
| MCP Tool-Layer Mapping Under the Shared Descriptor Contract | M2 does not change MCP runtime transport behavior | Full MCP discovery/runtime regression suite in `cargo test`; `tests/mcp_native_regression.rs > native_tool_registry_is_stable_with_mcp_enabled_or_disabled` | ✅ COMPLIANT |
| M2 Security and Entry-Point Parity Preservation | Descriptor identity preserves approval and profile behavior | `src/bootstrap/mod.rs > registry_ids_preserve_existing_profile_and_mcp_prefix_classification` | ✅ COMPLIANT |
| M2 Security and Entry-Point Parity Preservation | Agent, channel, and gateway parity remains unchanged in M2 | `src/bootstrap/mod.rs > gateway_bootstrap_registry_matches_final_active_tool_set`; broad existing agent/channel/gateway suites still pass | ⚠️ PARTIAL |
| M2 Anti-Scope and Deferred Work Constraints | Dependency resolution remains deferred beyond M2 | `src/capabilities/tool_registration.rs > builds_native_tool_descriptor_with_defaults`; no resolver code introduced | ✅ COMPLIANT |
| M2 Anti-Scope and Deferred Work Constraints | Registry-driven dispatch remains out of scope in M2 | Static evidence: `src/agent/agent.rs` still executes via `self.tools.iter().find(...)`; full `cargo test` passes | ✅ COMPLIANT |
| M2 Anti-Scope and Deferred Work Constraints | Non-tool families remain deferred beyond M2 | Type/module scope constrained to tool-family only; no dedicated runtime test | ⚠️ PARTIAL |

**Compliance summary**: 14/19 scenarios compliant, 5/19 partial, 0 failing, 0 untested-critical.

---

### Correctness (Static — Structural Evidence)
| Requirement | Status | Notes |
|------------|--------|-------|
| Descriptive registry exists | ✅ Implemented | New `capabilities/` module and `BootstrapContext.capability_registry`. |
| Descriptor minimums | ✅ Implemented | Shared fields, M2 defaults, MCP metadata, and validation present. |
| Registration timing after final tool selection | ✅ Implemented | Registry built after `profile.allows_tool(tool.name())` filtering in bootstrap. |
| Deterministic validation / uniqueness | ✅ Implemented | `CapabilityRegistry` uses ordered storage and explicit duplicate/field validation. |
| MCP mapping under shared descriptor contract | ✅ Implemented | Native/MCP tool/resource/prompt descriptor builders exist and use canonical ids. |
| Security / parity preservation | ✅ Implemented | Existing ids and `mcp.` prefixes are preserved; execution still uses tool vector. |
| M2 anti-scope boundaries | ✅ Implemented | No dependency resolution, no registry-driven dispatch, no non-tool family rollout found. |

---

### Coherence (Design)
| Decision | Followed? | Notes |
|----------|-----------|-------|
| Keep registry non-executing and bootstrap-owned | ✅ Yes | Registry attached to bootstrap context and built from final tool set. |
| Reuse current tool ids as descriptor ids | ✅ Yes | Descriptor ids use `tool.spec().name` / existing canonical MCP names. |
| Treat all M2 registrations as tool-family descriptors | ✅ Yes | Only `CapabilityFamily::Tool` exists in M2. |
| Use explicit descriptor builders instead of `ToolSpec` alone | ✅ Yes | `tool_registration.rs` combines `ToolSpec` and `descriptor_hint()`. |
| Preserve runtime merge behavior separately | ⚠️ Deviated slightly | Collision behavior is preserved behaviorally by building from the already-filtered runtime tool vector; direct registry duplicate tests still fail fast for invalid standalone descriptor sets. |

---

### Issues Found

**CRITICAL** (must fix before archive):
None.

**WARNING** (should fix):
- No dedicated runtime test proves provider/channel payload generation is registry-independent; current evidence is indirect through unchanged code paths and passing existing suites.
- Native/MCP collision preservation is validated indirectly (legacy runtime + standalone registry duplicate test), not with a single bootstrap-level collision reflection test.
- Some scope-boundary scenarios are enforced primarily by type/module shape rather than dedicated runtime tests.

**SUGGESTION** (nice to have):
- Add one focused bootstrap/integration regression covering a native/MCP collision case and asserting the registry reflects the post-skip runtime set.
- Add one explicit channel/provider regression asserting the registry is never consulted for tool payload emission.

---

### Verdict
PASS WITH WARNINGS

Implementation is behaviorally acceptable for M2, stays within scope, and has sufficient runtime evidence to proceed, but a few scenarios are only partially covered by indirect regression tests rather than dedicated new runtime tests.
