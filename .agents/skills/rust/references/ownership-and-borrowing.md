# Ownership and Borrowing

## Core rule

Make ownership obvious. If a function only reads, borrow. If it stores or transfers, own.

## Practical heuristics

- Use `&str` instead of `String` for read-only text inputs.
- Use `&Path` instead of `PathBuf` for path readers/validators.
- Use slices (`&[T]`) for read-only collections.
- Prefer returning owned data only when the caller needs to keep it.
- Clone at thread/task boundaries only when borrowing is impractical.

## Smells

- Cloning near every function call
- Passing `String`/`Vec<T>` when only reading
- Reaching for `Arc<Mutex<T>>` before proving shared mutable state is necessary
- Fighting the borrow checker with architecture that hides data flow

## Better pattern

```rust
fn is_allowed(command: &str, allowlist: &[String]) -> bool {
    allowlist.iter().any(|entry| entry == command)
}
```
