#!/usr/bin/env bash
set -euo pipefail

echo "Running diagnostics..."

if [ -f .gitmodules ]; then
  echo "✅ Git submodules detected"
else
  echo "ℹ️  No git submodules"
fi

if command -v docker >/dev/null 2>&1; then
  docker --version || echo "⚠️  Warning: docker --version failed"
else
  echo "ℹ️  Docker not installed"
fi

if command -v pnpm >/dev/null 2>&1; then
  pnpm --version || echo "⚠️  Warning: pnpm --version failed"
else
  echo "ℹ️  pnpm not installed"
fi

if command -v rustup >/dev/null 2>&1; then
  rustup show active-toolchain || echo "⚠️  Warning: rustup show active-toolchain failed"
else
  echo "ℹ️  rustup not installed"
fi

echo "Diagnostics complete"
