#!/usr/bin/env bash
set -euo pipefail

case "${OS:-}" in
  Windows_NT)
    if ! command -v bash >/dev/null 2>&1; then
      echo "Error: bash is required on Windows." >&2
      echo "Install Git for Windows or enable WSL, then retry." >&2
      exit 1
    fi
    ;;
esac
