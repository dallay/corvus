#!/usr/bin/env bash
set -euo pipefail

require_cmd() {
  local cmd_name="$1"
  command -v "$cmd_name" >/dev/null 2>&1 || {
    echo "Error: '$cmd_name' is not installed." >&2
    exit 1
  }
  return 0
}

echo "Checking required tools and versions..."

require_cmd java
require_cmd git
require_cmd node
require_cmd pnpm
require_cmd rustc
require_cmd cargo

java_ver_raw=$(java -version 2>&1 | awk -F '"' '/version/ {print $2; exit}')
java_major=${java_ver_raw%%.*}
if [[ "$java_major" = "1" ]]; then
  java_major=${java_ver_raw#1.}
  java_major=${java_major%%.*}
fi

node_major=$(node -p "process.versions.node.split('.')[0]")
pnpm_major=$(pnpm --version | awk -F. '{print $1}')

rust_full_ver=$(rustc --version | awk '{print $2}')
rust_major=${rust_full_ver%%.*}
rust_minor_part=${rust_full_ver#*.}
rust_minor=${rust_minor_part%%.*}

if [[ -z "$java_major" || "$java_major" -lt 21 ]]; then
  echo "Error: JDK 21+ required. Found: ${java_major:-unknown}" >&2
  exit 1
fi

if [[ "$node_major" -lt 22 ]]; then
  echo "Error: Node.js 22+ required. Found: $node_major" >&2
  exit 1
fi

if [[ "$pnpm_major" -lt 10 ]]; then
  echo "Error: pnpm 10+ required. Found: $pnpm_major" >&2
  exit 1
fi

if [[ "$rust_major" -lt 1 || ( "$rust_major" -eq 1 && "$rust_minor" -lt 75 ) ]]; then
  echo "Error: Rust 1.75+ required. Found: $rust_full_ver" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "Warning: Docker is not installed (optional; required for sandbox/dev containers)."
fi

if [[ "$(uname -s 2>/dev/null || echo unknown)" = "Darwin" ]] && ! command -v xcodebuild >/dev/null 2>&1; then
  echo "Warning: Xcode CLI tools not found (optional; required for iOS development)."
fi

echo "Toolchain OK: java=$java_major, node=$node_major, pnpm=$pnpm_major, rustc=$rust_full_ver"
