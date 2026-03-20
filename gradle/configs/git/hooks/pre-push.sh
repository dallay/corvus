#!/bin/sh
set -e

if [ "${SKIP_GIT_HOOKS:-0}" = "1" ]; then
    echo "⏭️  Skipping pre-push hook (SKIP_GIT_HOOKS=1)"
    exit 0
fi

echo "🚀 Pre-push check start"

RUST_RUNTIME_DIR="clients/agent-runtime"

if [ -d "$RUST_RUNTIME_DIR" ]; then
    echo "🦀 Running Rust runtime checks..."
    (
        cd "$RUST_RUNTIME_DIR"
        cargo fmt --check
        cargo clippy -- -D warnings
        cargo test
    )
fi

# Check if pnpm is available in the current PATH
if command -v pnpm >/dev/null 2>&1; then
    echo "✅ pnpm found, running full check..."
    ./gradlew check
else
    echo "⚠️  WARNING: pnpm not found in PATH"
    echo "   Skipping web checks that require pnpm, running core validations only..."
    echo "   To enable full checks, ensure pnpm is available: corepack enable && corepack prepare pnpm@latest --activate"
    echo ""
    # Skip the Gradle web module checks when pnpm is unavailable.
    ./gradlew check -x :web:check
fi

echo "✅ Pre-push check passed"
