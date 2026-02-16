#!/bin/sh
set -e

echo "🚀 Pre-push check start"

RUST_RUNTIME_DIR="apps/agent-runtime"

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
    echo "   Skipping documentation checks, running core validations only..."
    echo "   To enable full checks, ensure pnpm is available: corepack enable && corepack prepare pnpm@latest --activate"
    echo ""
    # Run check excluding docs tasks that require pnpm
    ./gradlew check -x :docs:pnpmInstall -x :docs:websiteCheck -x :docs:fileContentCheck
fi

echo "✅ Pre-push check passed"
