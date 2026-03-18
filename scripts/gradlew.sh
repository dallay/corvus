#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ "${OS:-}" == "Windows_NT" ]]; then
  exec "$ROOT_DIR/gradlew.bat" "$@"
fi

exec "$ROOT_DIR/gradlew" "$@"
