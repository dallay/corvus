#!/bin/bash
set -e

# Detect execution context (root or dev/)
if [ -f "dev/docker-compose.yml" ]; then
    BASE_DIR="dev"
    HOST_TARGET_DIR="clients/agent-runtime/target"
elif [ -f "docker-compose.yml" ] && [ "$(basename "$(pwd)")" == "dev" ]; then
    BASE_DIR="."
    HOST_TARGET_DIR="../clients/agent-runtime/target"
else
    echo "❌ Error: Run this script from the project root or dev/ directory."
    exit 1
fi

COMPOSE_FILE="$BASE_DIR/docker-compose.yml"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

function ensure_config {
    CONFIG_DIR="$HOST_TARGET_DIR/.corvus"
    CONFIG_FILE="$CONFIG_DIR/config.toml"
    WORKSPACE_DIR="$CONFIG_DIR/workspace"

    if [ ! -f "$CONFIG_FILE" ]; then
        echo -e "${YELLOW}⚙️  Config file missing in $HOST_TARGET_DIR/.corvus. Creating default dev config from template...${NC}"
        mkdir -p "$WORKSPACE_DIR"

        # Copy template
        cat "$BASE_DIR/config.template.toml" > "$CONFIG_FILE"
    fi
}

function print_help {
    echo -e "${YELLOW}Corvus Development Environment Manager${NC}"
    echo "Usage: ./dev/cli.sh [command]"
    echo ""
    echo "Commands:"
    echo -e "  ${GREEN}up${NC}      Start dev environment (Agent + Sandbox)"
    echo -e "  ${GREEN}down${NC}    Stop containers"
    echo -e "  ${GREEN}shell${NC}   Enter Sandbox (Ubuntu)"
    echo -e "  ${GREEN}agent${NC}   Enter Agent (Corvus CLI)"
    echo -e "  ${GREEN}logs${NC}    View logs"
    echo -e "  ${GREEN}build${NC}   Rebuild images"
    echo -e "  ${GREEN}clean${NC}   Stop and wipe workspace data"
}

if [ -z "$1" ]; then
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

    clean)
        echo -e "${RED}⚠️  WARNING: This will delete '$HOST_TARGET_DIR/.corvus' data and Docker volumes.${NC}"
        read -p "Are you sure? (y/N) " -n 1 -r
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
