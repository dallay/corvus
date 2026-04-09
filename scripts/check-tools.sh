#!/usr/bin/env bash
set -euo pipefail

# Visuals
BOLD="$(tput bold 2>/dev/null || echo '')"
RESET="$(tput sgr0 2>/dev/null || echo '')"
GREEN="$(tput setaf 2 2>/dev/null || echo '')"
YELLOW="$(tput setaf 3 2>/dev/null || echo '')"
RED="$(tput setaf 1 2>/dev/null || echo '')"
CYAN="$(tput setaf 6 2>/dev/null || echo '')"

# Configuration
MIN_JAVA=21
MIN_NODE=22
MIN_PNPM=10
MIN_RUST_MAJOR=1
MIN_RUST_MINOR=75

echo "${BOLD}🔍 Checking Developer Environment...${RESET}"
echo "--------------------------------------"

# Results tracking
FAILED=0

# Helper to print status
# status <name> <result: 0|1|2> <version/msg>
# 0 = OK, 1 = Error, 2 = Warning
print_status() {
  local name="$1"
  local res="$2"
  local info="$3"

  case "$res" in
    0) printf "  %-12s [%s%s%s] %s\n" "$name" "$GREEN" "✅" "$RESET" "$info" ;;
    1) printf "  %-12s [%s%s%s] %s\n" "$name" "$RED" "❌" "$RESET" "${RED}${info}${RESET}" ; FAILED=1 ;;
    2) printf "  %-12s [%s%s%s] %s\n" "$name" "$YELLOW" "⚠️" "$RESET" "${YELLOW}${info}${RESET}" ;;
  esac
}

numeric_prefix() {
  local raw="$1"
  local num="${raw%%[!0-9]*}"
  echo "${num:-0}"
}

# 1. Java
if command -v java >/dev/null 2>&1; then
  java_ver_raw=$(java -version 2>&1 | awk -F '"' '/version/ {print $2; exit}')
  java_major=$(echo "$java_ver_raw" | cut -d. -f1)
  if [[ "$java_major" = "1" ]]; then
    java_major=$(echo "$java_ver_raw" | cut -d. -f2)
  fi
  java_major=$(numeric_prefix "$java_major")

  if [[ "$java_major" -lt "$MIN_JAVA" ]]; then
    print_status "Java" 1 "Found $java_ver_raw, need JDK $MIN_JAVA+"
  else
    print_status "Java" 0 "v$java_ver_raw"
  fi
else
  print_status "Java" 1 "Not installed (JDK $MIN_JAVA+ required)"
fi

# 2. Git
if command -v git >/dev/null 2>&1; then
  git_ver=$(git --version | awk '{print $3}')
  print_status "Git" 0 "v$git_ver"
else
  print_status "Git" 1 "Not installed"
fi

# 3. Node.js
if command -v node >/dev/null 2>&1; then
  node_full=$(node -v)
  node_major=$(numeric_prefix "${node_full#v}")
  if [[ "$node_major" -lt "$MIN_NODE" ]]; then
    print_status "Node.js" 1 "Found $node_full, need v$MIN_NODE+"
  else
    print_status "Node.js" 0 "$node_full"
  fi
else
  print_status "Node.js" 1 "Not installed (v$MIN_NODE+ required)"
fi

# 4. pnpm
if command -v pnpm >/dev/null 2>&1; then
  pnpm_ver=$(pnpm --version)
  pnpm_major=$(numeric_prefix "$pnpm_ver")
  if [[ "$pnpm_major" -lt "$MIN_PNPM" ]]; then
    print_status "pnpm" 1 "Found v$pnpm_ver, need v$MIN_PNPM+"
  else
    print_status "pnpm" 0 "v$pnpm_ver"
  fi
else
  print_status "pnpm" 1 "Not installed (v$MIN_PNPM+ required)"
fi

# 5. Rust
if command -v rustc >/dev/null 2>&1; then
  rust_full=$(rustc --version | awk '{print $2}')
  rust_major=$(numeric_prefix "${rust_full%%.*}")
  rust_minor_part=${rust_full#*.}
  rust_minor=$(numeric_prefix "${rust_minor_part%%.*}")

  if [[ "$rust_major" -lt "$MIN_RUST_MAJOR" || ( "$rust_major" -eq "$MIN_RUST_MAJOR" && "$rust_minor" -lt "$MIN_RUST_MINOR" ) ]]; then
    print_status "Rust" 1 "Found v$rust_full, need v$MIN_RUST_MAJOR.$MIN_RUST_MINOR+"
  else
    print_status "Rust" 0 "v$rust_full"
  fi
else
  print_status "Rust" 1 "Not installed (v$MIN_RUST_MAJOR.$MIN_RUST_MINOR+ required)"
fi

# 6. Docker (Optional)
if command -v docker >/dev/null 2>&1; then
  docker_ver=$(docker --version | awk '{print $3}' | tr -d ',')
  print_status "Docker" 0 "v$docker_ver"
else
  print_status "Docker" 2 "Optional; required for sandbox/dev containers"
fi

echo "--------------------------------------"
if [[ "$FAILED" -eq 0 ]]; then
  echo "${GREEN}${BOLD}✅ Toolchain is ready for Corvus development!${RESET}"
  exit 0
else
  echo "${RED}${BOLD}❌ Missing or incompatible requirements found.${RESET}"
  echo "Please install the missing tools and try again."
  exit 1
fi
