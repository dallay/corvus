#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/clients/agent-runtime/docker-compose.yml"

command_name="${1:-}"
shift || true

case "$command_name" in
  up)
    exec docker compose -f "$COMPOSE_FILE" up -d "$@"
    ;;
  up-dashboard)
    exec docker compose -f "$COMPOSE_FILE" --profile dashboard up -d "$@"
    ;;
  down)
    exec docker compose -f "$COMPOSE_FILE" --profile dashboard down "$@"
    ;;
  logs)
    exec docker compose -f "$COMPOSE_FILE" --profile dashboard logs -f "$@"
    ;;
  status)
    exec docker compose -f "$COMPOSE_FILE" --profile dashboard ps "$@"
    ;;
  *)
    echo "Usage: bash scripts/runtime-compose.sh {up|up-dashboard|down|logs|status}" >&2
    exit 1
    ;;
esac
