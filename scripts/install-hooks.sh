#!/usr/bin/env bash
set -euo pipefail

# Visuals
BOLD="$(tput bold 2>/dev/null || echo '')"
RESET="$(tput sgr0 2>/dev/null || echo '')"
GREEN="$(tput setaf 2 2>/dev/null || echo '')"
CYAN="$(tput setaf 6 2>/dev/null || echo '')"

HOOKS_SRC="gradle/configs/git/hooks"
HOOKS_DEST=".git/hooks"

echo "${BOLD}🔗 Installing Git Hooks...${RESET}"

if [ ! -d ".git" ]; then
  echo "❌ Error: .git directory not found. Are you in the repository root?"
  exit 1
fi

mkdir -p "$HOOKS_DEST"

# Iterate over all scripts in the source hooks directory
for hook_path in "$HOOKS_SRC"/*.sh; do
  hook_name=$(basename "$hook_path" .sh)
  dest_path="$HOOKS_DEST/$hook_name"

  echo "  - Installing ${CYAN}$hook_name${RESET} hook..."

  # Copy the hook and ensure it is executable
  cp "$hook_path" "$dest_path"
  chmod +x "$dest_path"
done

echo "${GREEN}${BOLD}✅ Git hooks installed successfully!${RESET}"
exit 0
