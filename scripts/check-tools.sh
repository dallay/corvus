#!/usr/bin/env bash
set -euo pipefail

# Visuals
BOLD="$(tput bold 2>/dev/null || echo '')"
RESET="$(tput sgr0 2>/dev/null || echo '')"
GREEN="$(tput setaf 2 2>/dev/null || echo '')"
YELLOW="$(tput setaf 3 2>/dev/null || echo '')"
RED="$(tput setaf 1 2>/dev/null || echo '')"

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

# print_status <name> <result> <info>
#
# Print a formatted status line for a tool check.
#
# Parameters:
#   $1 (name)   Tool/check name to display.
#   $2 (result) Status code:
#                 0 = OK
#                 1 = Error
#                 2 = Warning
#   $3 (info)   Version string or descriptive message to display.
#
# Returns:
#   0 when result is OK or Warning (res=0 or res=2).
#   1 when result is Error (res=1).
#   Caller can use `|| FAILED=1` to accumulate failures.
print_status() {
  local name="$1"
  local res="$2"
  local info="$3"

  case "$res" in
    0) printf "  %-12s [%s%s%s] %s\n" "$name" "$GREEN" "✅" "$RESET" "$info" ;;
    1) printf "  %-12s [%s%s%s] %s\n" "$name" "$RED" "❌" "$RESET" "${RED}${info}${RESET}" ; return 1 ;;
    2) printf "  %-12s [%s%s%s] %s\n" "$name" "$YELLOW" "⚠️" "$RESET" "${YELLOW}${info}${RESET}" ;;
  esac
  return 0
}

# Extract the leading numeric portion of a version token.
# Returns "0" when the input does not start with any digits.
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
    print_status "Java" 1 "Found $java_ver_raw, need JDK $MIN_JAVA+" || FAILED=1
  else
    print_status "Java" 0 "v$java_ver_raw"
  fi
else
  print_status "Java" 1 "Not installed (JDK $MIN_JAVA+ required)" || FAILED=1
fi

# 2. Git
if command -v git >/dev/null 2>&1; then
  git_ver=$(git --version | awk '{print $3}')
  print_status "Git" 0 "v$git_ver"
else
  print_status "Git" 1 "Not installed" || FAILED=1
fi

# 3. Node.js
# check_required_major_version <tool_name> <current_version> <min_major>
#
# Check that <current_version> meets the minimum major version requirement.
#
# Parameters:
#   $1 (tool_name)        Display name of the tool.
#   $2 (current_version)  Full version string of the installed tool (without leading 'v').
#   $3 (min_major)        Minimum required major version number.
#
# Side effects:
#   Prints a status line via print_status and sets FAILED=1 on version mismatch.
check_required_major_version() {
  local tool_name="$1"
  local current_version="$2"
  local min_major="$3"
  local current_major

  current_major=$(numeric_prefix "${current_version%%.*}")
  if [[ "$current_major" -lt "$min_major" ]]; then
    print_status "$tool_name" 1 "Found v$current_version, need v$min_major+" || FAILED=1
  else
    print_status "$tool_name" 0 "v$current_version"
  fi
}

# check_required_major_minor_version <tool_name> <current_version> <min_major> <min_minor>
#
# Check that <current_version> meets the minimum major.minor version requirement.
#
# Parameters:
#   $1 (tool_name)        Display name of the tool.
#   $2 (current_version)  Full version string of the installed tool (without leading 'v').
#   $3 (min_major)        Minimum required major version number.
#   $4 (min_minor)        Minimum required minor version number when major equals min_major.
#
# Side effects:
#   Prints a status line via print_status and sets FAILED=1 on version mismatch.
check_required_major_minor_version() {
  local tool_name="$1"
  local current_version="$2"
  local min_major="$3"
  local min_minor="$4"
  local current_major
  local minor_part
  local current_minor

  current_major=$(numeric_prefix "${current_version%%.*}")
  minor_part=${current_version#*.}
  current_minor=$(numeric_prefix "${minor_part%%.*}")

  if [[ "$current_major" -lt "$min_major" || ( "$current_major" -eq "$min_major" && "$current_minor" -lt "$min_minor" ) ]]; then
    print_status "$tool_name" 1 "Found v$current_version, need v$min_major.$min_minor+" || FAILED=1
  else
    print_status "$tool_name" 0 "v$current_version"
  fi
}

if command -v node >/dev/null 2>&1; then
  node_full=$(node -v)
  check_required_major_version "Node.js" "${node_full#v}" "$MIN_NODE"
else
  print_status "Node.js" 1 "Not installed (v$MIN_NODE+ required)" || FAILED=1
fi

# 4. pnpm
if command -v pnpm >/dev/null 2>&1; then
  pnpm_ver=$(pnpm --version)
  check_required_major_version "pnpm" "$pnpm_ver" "$MIN_PNPM"
else
  print_status "pnpm" 1 "Not installed (v$MIN_PNPM+ required)" || FAILED=1
fi

# 5. Rust
if command -v rustc >/dev/null 2>&1; then
  rust_full=$(rustc --version | awk '{print $2}')
  check_required_major_minor_version "Rust" "$rust_full" "$MIN_RUST_MAJOR" "$MIN_RUST_MINOR"
else
  print_status "Rust" 1 "Not installed (v$MIN_RUST_MAJOR.$MIN_RUST_MINOR+ required)" || FAILED=1
fi

# 6. Docker (Optional)
if command -v docker >/dev/null 2>&1; then
  docker_ver=$(docker --version | awk '{print $3}' | tr -d ',')
  print_status "Docker" 0 "v$docker_ver"
else
  print_status "Docker" 2 "Optional; required for sandbox/dev containers"
fi

# 7. Xcode CLI Tools (macOS only, optional)
if [[ "$(uname -s 2>/dev/null || echo unknown)" = "Darwin" ]]; then
  if command -v xcodebuild >/dev/null 2>&1; then
    xcode_ver=$(xcodebuild -version 2>/dev/null | awk 'NR==1{print $2}')
    print_status "Xcode" 0 "v${xcode_ver:-unknown}"
  else
    print_status "Xcode" 2 "Optional; required for iOS development"
  fi
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
