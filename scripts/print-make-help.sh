#!/usr/bin/env bash
set -euo pipefail

makefiles=("$@")

bold=""
reset=""
cyan=""

if command -v tput >/dev/null 2>&1; then
  bold="$(tput bold 2>/dev/null || true)"
  reset="$(tput sgr0 2>/dev/null || true)"
  cyan="$(tput setaf 6 2>/dev/null || true)"
fi

printf '%s\n' "${bold}CORVUS - MONOREPO COMMAND CENTER${reset}"
printf '\n%s\n' "${bold}Usage:${reset} make ${cyan}[target]${reset}"
printf '%s\n' "${bold}Quick Start:${reset}"
printf '  %smake run%s           - Run the main Desktop application\n' "$cyan" "$reset"
printf '  %smake setup%s         - Initial project setup and tool validation\n' "$cyan" "$reset"
printf '  %smake build%s         - Build the entire project\n' "$cyan" "$reset"
printf '  %smake test%s          - Run all project tests\n' "$cyan" "$reset"
printf '\n%s\n' "${bold}Available Commands:${reset}"

while IFS= read -r line; do
  if [[ "$line" =~ ^#\ ---\ (.*)\ ---$ ]]; then
    printf '\n%s%s%s\n' "$bold" "${BASH_REMATCH[1]}" "$reset"
    continue
  fi

  if [[ "$line" =~ ^([a-zA-Z0-9_-]+):.*##\ (.*)$ ]]; then
    printf '  %s%-20s%s %s\n' "$cyan" "${BASH_REMATCH[1]}" "$reset" "${BASH_REMATCH[2]}"
  fi
done < <(cat "${makefiles[@]}")
