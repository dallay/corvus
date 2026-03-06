#!/bin/bash
set -e

# Detect execution context (root or dev/)
if [[ -f "dev/docker-compose.yml" ]]; then
    BASE_DIR="dev"
    HOST_TARGET_DIR="clients/agent-runtime/target"
elif [[ -f "docker-compose.yml" ]] && [[ "$(basename "$(pwd)")" == "dev" ]]; then
    BASE_DIR="."
    HOST_TARGET_DIR="../clients/agent-runtime/target"
else
    echo "❌ Error: Run this script from the project root or dev/ directory." >&2
    exit 1
fi

COMPOSE_FILE="$BASE_DIR/docker-compose.yml"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

function wait_http_ok {
    local url="$1"
    local timeout_secs="$2"
    local start_ts
    start_ts="$(date +%s)"

    while true; do
        if curl -fsS "$url" > /dev/null 2>&1; then
            return 0
        fi

        local now_ts
        now_ts="$(date +%s)"
        if (( now_ts - start_ts >= timeout_secs )); then
            return 1
        fi
        sleep 1
    done
}

function ensure_config {
    CONFIG_DIR="$HOST_TARGET_DIR/.corvus"
    CONFIG_FILE="$CONFIG_DIR/config.toml"
    WORKSPACE_DIR="$CONFIG_DIR/workspace"

    if [[ ! -f "$CONFIG_FILE" ]]; then
        echo -e "${YELLOW}⚙️  Config file missing in $HOST_TARGET_DIR/.corvus. Creating default dev config from template...${NC}"
        mkdir -p "$WORKSPACE_DIR"

        # Copy template
        cat "$BASE_DIR/config.template.toml" > "$CONFIG_FILE"
    fi

    return 0
}

function print_help {
    echo -e "${YELLOW}Corvus Development Environment Manager${NC}"
    echo "Usage: ./dev/cli.sh [command]"
    echo ""
    echo "Commands:"
    echo -e "  ${GREEN}up${NC}                Start dev environment (Agent + Sandbox)"
    echo -e "  ${GREEN}up-dashboard${NC}      Start dev environment + Dashboard"
    echo -e "  ${GREEN}down${NC}    Stop containers"
    echo -e "  ${GREEN}shell${NC}   Enter Sandbox (Ubuntu)"
    echo -e "  ${GREEN}agent${NC}   Enter Agent (Corvus CLI)"
    echo -e "  ${GREEN}logs${NC}    View logs"
    echo -e "  ${GREEN}build${NC}             Rebuild agent + sandbox images"
    echo -e "  ${GREEN}build-dashboard${NC}   Rebuild dashboard image"
    echo -e "  ${GREEN}smoke${NC}             Quick health checks (gateway + optional dashboard)"
    echo -e "  ${GREEN}clean${NC}   Stop and wipe workspace data"

    return 0
}

if [[ -z "$1" ]]; then
    print_help
    exit 1
fi

case "$1" in
    up)
        ensure_config
        echo -e "${GREEN}🚀 Starting Dev Environment...${NC}"
        # Build context MUST be set correctly for docker compose
        docker compose -f "$COMPOSE_FILE" up -d
        echo -e "${GREEN}✅ Environment is running!${NC}"
        echo -e "   - Agent: http://127.0.0.1:3000"
        echo -e "   - Sandbox: running (background)"
        echo -e "   - Config: $HOST_TARGET_DIR/.corvus/config.toml (Edit locally to apply changes)"
        ;;

    up-dashboard)
        ensure_config
        echo -e "${GREEN}🚀 Starting Dev Environment (with Dashboard)...${NC}"
        docker compose -f "$COMPOSE_FILE" --profile dashboard up -d
        echo -e "${GREEN}✅ Environment is running!${NC}"
        echo -e "   - Agent: http://127.0.0.1:3000"
        echo -e "   - Dashboard: http://127.0.0.1:4324"
        echo -e "   - Sandbox: running (background)"
        echo -e "   - Config: $HOST_TARGET_DIR/.corvus/config.toml (Edit locally to apply changes)"
        ;;

    down)
        echo -e "${YELLOW}🛑 Stopping services...${NC}"
        docker compose -f "$COMPOSE_FILE" down
        echo -e "${GREEN}✅ Stopped.${NC}"
        ;;

    shell)
        echo -e "${GREEN}💻 Entering Sandbox (Ubuntu)... (Type 'exit' to leave)${NC}"
        docker exec -it corvus-sandbox /bin/bash
        ;;

    agent)
        echo -e "${GREEN}🤖 Entering Agent Container (Corvus)... (Type 'exit' to leave)${NC}"
        docker exec -it corvus-dev /bin/bash
        ;;

    logs)
        docker compose -f "$COMPOSE_FILE" logs -f
        ;;

    build)
        echo -e "${YELLOW}🔨 Rebuilding images...${NC}"
        docker compose -f "$COMPOSE_FILE" build
        ensure_config
        docker compose -f "$COMPOSE_FILE" up -d
        echo -e "${GREEN}✅ Rebuild complete.${NC}"
        ;;

    build-dashboard)
        echo -e "${YELLOW}🔨 Rebuilding dashboard image...${NC}"
        docker compose -f "$COMPOSE_FILE" --profile dashboard build dashboard-dev
        echo -e "${GREEN}✅ Dashboard rebuild complete.${NC}"
        ;;

    smoke)
        echo -e "${YELLOW}🧪 Running smoke checks...${NC}"

        if wait_http_ok "http://127.0.0.1:3000/health" 30; then
            echo -e "${GREEN}✅ Gateway healthy:${NC} http://127.0.0.1:3000/health"
        else
            echo -e "${RED}❌ Gateway check failed:${NC} http://127.0.0.1:3000/health"
            echo -e "   Hint: start with './dev/cli.sh up' or './dev/cli.sh up-dashboard'"
            exit 1
        fi

        RUNNING_SERVICES="$(docker compose -f "$COMPOSE_FILE" ps --services --status running || true)"
        if echo "$RUNNING_SERVICES" | grep -q "^dashboard-dev$"; then
            if wait_http_ok "http://127.0.0.1:4324" 30; then
                echo -e "${GREEN}✅ Dashboard reachable:${NC} http://127.0.0.1:4324"
            else
                echo -e "${RED}❌ Dashboard check failed:${NC} http://127.0.0.1:4324"
                echo -e "   Hint: check logs with './dev/cli.sh logs'"
                exit 1
            fi
        else
            echo -e "${YELLOW}ℹ️  Dashboard not running (profile not enabled).${NC}"
            echo -e "   Start it with './dev/cli.sh up-dashboard'"
        fi

        echo -e "${GREEN}✅ Smoke checks passed.${NC}"
        ;;

    clean)
        echo -e "${RED}⚠️  WARNING: This will delete '$HOST_TARGET_DIR/.corvus' data and Docker volumes.${NC}"
        read -r -n 1 -p "Are you sure? (y/N) " REPLY
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            docker compose -f "$COMPOSE_FILE" down -v
            rm -rf "$HOST_TARGET_DIR/.corvus"
            echo -e "${GREEN}🧹 Cleaned up (playground/ remains intact).${NC}"
        else
            echo "Cancelled."
        fi
        ;;

    *)
        print_help
        exit 1
        ;;
esac
