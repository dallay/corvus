#!/usr/bin/env bash
set -e

echo "🔍 Checking latest commit message..."

# ------------------------------
# ANSI colors
# ------------------------------
RESET="\033[0m"
RED="\033[31m"
GREEN="\033[32m"
BLUE="\033[34m"
BG_RED="\033[41m"

# ------------------------------
# Get latest commit message
# ------------------------------
MSG_PATH="$1"

if [[ ! -f "$MSG_PATH" ]]; then
  echo "ERROR: commit message file not found: $MSG_PATH" >&2
  exit 1
fi

COMMIT_MSG="$(awk 'BEGIN{header=""} /^[[:space:]]*#/ {next} {header=$0; print header; exit}' "$MSG_PATH")"
echo -e "📝 Commit message header:\n  ${GREEN}${COMMIT_MSG}${RESET}\n"

# ------------------------------
# Commit message pattern
# ------------------------------
COMMIT_MSG_PATTERN='^(revert: )?(build|chore|ci|deps|docs|feat|fix|infra|perf|refactor|release|style|test|wip)(\([^)]+\))?(!)?: [^[:cntrl:]]{1,100}[^[:space:][:cntrl:]]$'

# ------------------------------
# Skip merge or initial commit
# ------------------------------
if echo "$COMMIT_MSG" | grep -Eq '^Merge'; then
  echo "⏭ Skipping merge commit."
  exit 0
fi

if echo "$COMMIT_MSG" | grep -Eq '^Initial commit'; then
  echo "⏭ Skipping initial commit."
  exit 0
fi

if ! echo "$COMMIT_MSG" | grep -Eq "$COMMIT_MSG_PATTERN"; then
  echo -e "${BG_RED}ERROR${RESET}  ${RED}invalid commit message format.${RESET}\n" >&2
  echo -e "${RED}Proper commit message format is required for automated changelog generation. Examples:${RESET}\n" >&2
  echo -e "  ${GREEN}feat(parser): add support for empty tuples${RESET}" >&2
  echo -e "  ${GREEN}fix(runtime): handle reconnect race condition${RESET}" >&2
  echo -e "  ${GREEN}refactor(core)!: remove legacy provider fallback${RESET}\n" >&2
  echo -e "${RED}Commit message header: <type>(<scope>): <subject>${RESET}" >&2
  echo -e "${RED}Commit message header pattern: ${COMMIT_MSG_PATTERN}${RESET}" >&2
  echo -e "${RED}See${RESET} ${BLUE}https://www.conventionalcommits.org/en/v1.0.0/${RESET} ${RED}for more details.${RESET}\n" >&2
  echo -e "${RED}❌ Invalid commit message:${RESET} '${COMMIT_MSG}'" >&2
  exit 1
fi

echo -e "${GREEN}✅ Latest commit message is valid.${RESET}"
