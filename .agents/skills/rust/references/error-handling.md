# Error Handling

## Prefer explicit failures

Rust code should make failure modes visible.

- Use `Result<T, E>` for recoverable errors.
- Use typed errors for library/runtime boundaries.
- Add context where orchestration would otherwise lose the source of failure.
- Reserve panics for truly impossible states or tests.

## Error layering

- Low-level modules: typed errors with actionable variants
- App/service orchestration: wrap with context
- CLI/top-level: present clean human-readable failures

## Example

```rust
#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("invalid header format")]
    InvalidHeader,
}

fn parse_header(value: &str) -> Result<&str, ParseError> {
    value.split_once(':').map(|(_, body)| body).ok_or(ParseError::InvalidHeader)
}
```
