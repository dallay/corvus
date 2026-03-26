# Runtime Architecture

## Mental model

Corvus runtime is organized around replaceable traits plus factory wiring.

### Extension points

- `src/providers/traits.rs`
- `src/channels/traits.rs`
- `src/tools/traits.rs`
- `src/memory/traits.rs`
- `src/observability/traits.rs`
- `src/runtime/traits.rs`

## Preferred workflow for new capability

1. Inspect the relevant trait
2. Implement it in the matching module tree
3. Register it in the local `mod.rs` / factory entrypoint
4. Add focused tests for success and failure paths
5. Run the smallest relevant validation commands

## Rule of thumb

If the behavior is provider-like/channel-like/tool-like/memory-like, do not create a parallel system.
Fit the existing contract first.
