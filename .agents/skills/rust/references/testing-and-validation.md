# Testing and Validation

## Default flow

1. Add or update the smallest failing test
2. Implement the minimum change to pass
3. Refactor safely
4. Run the smallest relevant validation commands

## Test placement

- `#[cfg(test)]` for pure module logic
- integration tests when behavior crosses modules or public contracts
- benchmarks only for measured hotspots

## Validation baseline

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```
