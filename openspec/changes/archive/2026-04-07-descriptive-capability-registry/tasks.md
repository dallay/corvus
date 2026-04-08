# Tasks: Descriptive Capability Registry

## Phase 1: Foundation

- [x] 1.1 Create
  `clients/agent-runtime/src/capabilities/{mod.rs,descriptor.rs,registry.rs,tool_registration.rs}`
  and export the new module from `src/lib.rs`/module wiring as needed.
- [x] 1.2 Add minimal M2 descriptor types in `descriptor.rs`: tool-only family/kind enums,
  lifecycle/security/compatibility/dependency structs, MCP metadata structs, and `CapabilityError`.
- [x] 1.3 Add `CapabilityRegistry` in `registry.rs` with deterministic storage,
  `register/get/iter/len/from_descriptors`, and explicit validation helpers.

## Phase 2: Test-first validation and collision rules

- [x] 2.1 RED: add unit tests in `capabilities/registry.rs` for missing required fields, invalid
  namespace, duplicate ids, deterministic iteration order, and explicit native/MCP collision policy.
- [x] 2.2 GREEN: implement descriptor completeness validation and uniqueness enforcement so the
  Phase 2 registry tests pass.
- [x] 2.3 REFACTOR: lock the chosen collision policy into error messages/types and keep registry
  behavior descriptive-only (no dedupe, no execution hooks).

## Phase 3: Descriptor builders and mapping

- [x] 3.1 RED: add builder tests in `capabilities/tool_registration.rs` for native tool descriptors
  using current tool ids, default metadata, and empty-but-present dependency fields.
- [x] 3.2 GREEN: implement native descriptor mapping from `&dyn Tool`/`ToolSpec` with exact ids and
  deterministic M2 defaults.
- [x] 3.3 RED: add MCP mapping tests for tool/resource/prompt descriptors covering canonical names,
  source classification, URI/mime-type mapping, and prompt arguments.
- [x] 3.4 GREEN: expose minimal metadata getters in
  `clients/agent-runtime/src/tools/mcp/{adapter.rs,resource_adapter.rs,prompt_adapter.rs}` and
  implement MCP descriptor builders in `tool_registration.rs`.

## Phase 4: Bootstrap integration

- [x] 4.1 RED: add bootstrap tests in `clients/agent-runtime/src/bootstrap/mod.rs` proving registry
  finalization happens after profile filtering and matches the final active tool set exactly.
- [x] 4.2 GREEN: add `capability_registry` to `BootstrapContext` and build it after
  `profile.allows_tool(tool.name())` filtering in both bootstrap entry paths.
- [x] 4.3 Verify `clients/agent-runtime/src/agent/agent.rs`, `src/channels/mod.rs`, and provider
  paths remain execution-spec based only; do not route dispatch/lookup through the registry.

## Phase 5: Parity tests and validation

- [x] 5.1 Add non-regression tests proving current tool ids and `mcp.` prefixes remain unchanged for
  approval/profile classification after registry introduction.
- [x] 5.2 Run the smallest relevant Rust tests for capability, MCP, and bootstrap modules first,
  then `cargo test --manifest-path clients/agent-runtime/Cargo.toml` if targeted coverage is clean.
- [x] 5.3 Run `cargo fmt --all -- --check` and
  `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings` only
  after code is stable; fix any issues without widening scope.
