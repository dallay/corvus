# Validation Commands

## Default runtime checks

```bash
make rust-fmt
make rust-clippy
make rust-test
```

## Direct cargo commands

```bash
cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check
cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path clients/agent-runtime/Cargo.toml
```

## Focused examples

```bash
cargo test --manifest-path clients/agent-runtime/Cargo.toml module_name::tests::test_name
cargo test --manifest-path clients/agent-runtime/Cargo.toml whatsapp_webhook_security
cargo bench --manifest-path clients/agent-runtime/Cargo.toml
```

## Validation rule

If you cannot run the relevant checks, say exactly what was skipped and why.
