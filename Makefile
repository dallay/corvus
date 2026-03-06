# ====================================================================================
# CORVUS MONOREPO - MAKEFILE
#
# Standardized commands for all developers and operating systems.
# Run `make help` to see all available commands.
# ====================================================================================

.DEFAULT_GOAL := help

# ------------------------------------------------------------------------------------
# VARIABLES & CONFIGURATION
# ------------------------------------------------------------------------------------

# Operating System Detection & Shell Normalization
ifeq ($(OS),Windows_NT)
    DETECTED_OS := Windows
    # Find bash.exe and convert to short path (8.3) to avoid space issues
    SHELL_PATH := $(shell for /f "delims=" %i in ('where bash.exe 2^>NUL') do @(for %j in ("%i") do @echo %~sj & exit /b 0))
    ifeq ($(SHELL_PATH),)
        $(error ❌ A bash-compatible shell (Git Bash, WSL) is required on Windows.)
    endif
    SHELL := $(SHELL_PATH)
else
    DETECTED_OS := $(shell uname -s 2>/dev/null || echo Unknown)
    SHELL := /bin/bash
endif

# Common Constants
GRADLEW := ./gradlew
DEV_NULL := /dev/null
MKDIR_P := mkdir -p

# Module Names
APP_MODULE   := :composeApp
RUST_MODULE  := :agent-runtime
WEB_MODULE   := :web
ANDROID_APP  := :androidApp
CORE_MODULE  := :agent-core-kmp

# ------------------------------------------------------------------------------------
# VISUALS & COLORS
# ------------------------------------------------------------------------------------
BOLD := $(shell tput bold 2>/dev/null || echo "")
SGR0 := $(shell tput sgr0 2>/dev/null || echo "")
CYAN := $(shell tput setaf 6 2>/dev/null || echo "")
GREEN := $(shell tput setaf 2 2>/dev/null || echo "")
YELLOW := $(shell tput setaf 3 2>/dev/null || echo "")
RED   := $(shell tput setaf 1 2>/dev/null || echo "")

# ------------------------------------------------------------------------------------
# CORE & HELP
# ------------------------------------------------------------------------------------

help: ## Show this help message
	@if [ "$(DETECTED_OS)" = "Windows" ]; then \
		echo "-----------------------------------------------------------------------"; \
		echo "                 CORVUS - MONOREPO COMMAND CENTER                      "; \
		echo "-----------------------------------------------------------------------"; \
	else \
		echo "$(BOLD)╔═══════════════════════════════════════════════════════════════════════╗$(SGR0)"; \
		echo "$(BOLD)║                 CORVUS - MONOREPO COMMAND CENTER                      ║$(SGR0)"; \
		echo "$(BOLD)╚═══════════════════════════════════════════════════════════════════════╝$(SGR0)"; \
	fi
	@echo ""
	@echo "$(BOLD)Usage:$(SGR0) make $(CYAN)[target]$(SGR0)"
	@echo ""
	@echo "$(BOLD)Quick Start:$(SGR0)"
	@echo "  $(CYAN)make run$(SGR0)           - Run the main Desktop application"
	@echo "  $(CYAN)make setup$(SGR0)         - Initial project setup and tool validation"
	@echo "  $(CYAN)make build$(SGR0)         - Build the entire project"
	@echo "  $(CYAN)make test$(SGR0)          - Run all project tests"
	@echo ""
	@echo "$(BOLD)Available Commands:$(SGR0)"
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$|^# --- .* ---$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; \
			/^# --- / { \
				section = $$0; \
				gsub(/^# --- /, "", section); \
				gsub(/ ---$$/, "", section); \
				printf "\n\033[1m%s\033[0m\n", section; \
				next; \
			} \
			/^[a-zA-Z0-9_-]+:/ { \
				printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2; \
			}'

h: help ## Alias for help

# --- ENVIRONMENT & SETUP ---

check-tools: ## Verify toolchain (Java 21, Node 22, pnpm 10, Rust 1.75)
	@echo "🔍 Checking required tools and versions..."
	@bash -ec ' \
		require_cmd() { \
			command -v "$$1" >/dev/null 2>&1 || { echo "$(RED)❌ Error: '\''$$1'\'' is not installed.$(SGR0)"; return 1; }; \
		}; \
		require_cmd java && require_cmd git && require_cmd node && require_cmd pnpm && require_cmd rustc && require_cmd cargo || exit 1; \
		java_ver=$$(java -version 2>&1 | sed -nE '\''s/.*version \"([0-9]+).*/\1/p'\'' | head -n1); \
		if [ -z "$$java_ver" ]; then java_ver=$$(java -version 2>&1 | head -n 1 | awk -F '\''"'\'' '\''{print $$2}'\'' | cut -d. -f1); fi; \
		node_ver=$$(node -p "process.versions.node.split(\".\")[0]"); \
		pnpm_ver=$$(pnpm --version | awk -F. '\''{print $$1}'\''); \
		rust_full_ver=$$(rustc --version | awk '\''{print $$2}'\''); \
		rust_major=$${rust_full_ver%%.*}; \
		rust_minor_part=$${rust_full_ver#*.}; \
		rust_minor=$${rust_minor_part%%.*}; \
		if [ -z "$$java_ver" ] || [ "$$java_ver" -lt 21 ]; then \
			echo "$(RED)❌ Error: JDK 21+ required. Found: $${java_ver:-unknown}$(SGR0)"; exit 1; \
		fi; \
		if [ "$$node_ver" -lt 22 ]; then \
			echo "$(RED)❌ Error: Node.js 22+ required. Found: $$node_ver$(SGR0)"; exit 1; \
		fi; \
		if [ "$$pnpm_ver" -lt 10 ]; then \
			echo "$(RED)❌ Error: pnpm 10+ required. Found: $$pnpm_ver$(SGR0)"; exit 1; \
		fi; \
		if [ "$$rust_major" -lt 1 ] || { [ "$$rust_major" -eq 1 ] && [ "$$rust_minor" -lt 75 ]; }; then \
			echo "$(RED)❌ Error: Rust 1.75+ required. Found: $$rust_full_ver$(SGR0)"; exit 1; \
		fi; \
		if ! command -v docker >/dev/null 2>&1; then \
			echo "$(YELLOW)⚠️  Docker is not installed (optional; required for sandbox/dev containers).$(SGR0)"; \
		fi; \
		if [ "$$(uname -s 2>/dev/null || echo unknown)" = "Darwin" ]; then \
			if ! command -v xcodebuild >/dev/null 2>&1; then \
				echo "$(YELLOW)⚠️  Xcode CLI tools not found (optional; required for iOS development).$(SGR0)"; \
			fi; \
		fi; \
		echo "$(GREEN)✅ Toolchain OK: java=$$java_ver, node=$$node_ver, pnpm=$$pnpm_ver, rustc=$$rust_full_ver$(SGR0)"; \
	'

setup: check-tools ## Initial project setup (agents, web deps, rust check)
	@echo "🔧 $(BOLD)Setting up project...$(SGR0)"
	@chmod +x gradlew
	@$(GRADLEW) agentsyncApply
	@$(GRADLEW) $(WEB_MODULE):workspaceInstall
	@$(GRADLEW) $(RUST_MODULE):cargoCheck -PenableRustTasks=true
	@echo "$(GREEN)✅ Project setup complete!$(SGR0)"

sync-agents: ## Sync AI agent configurations (agentsync)
	@$(GRADLEW) agentsyncApply

wrapper: ## Update Gradle wrapper
	@$(GRADLEW) wrapper --gradle-version $(shell grep -E '^gradle\s*=' gradle/libs.versions.toml | sed 's/.*= "\(.*\)".*/\1/')

# --- BUILD & CLEAN ---

build: check-tools ## Build the entire project
	@echo "🏗️  $(BOLD)Building project...$(SGR0)"
	@$(GRADLEW) build

build-fast: ## Build skipping tests
	@echo "🏗️  $(BOLD)Building project (skip tests)...$(SGR0)"
	@$(GRADLEW) build -x test

clean: ## Clean build artifacts
	@echo "🧹 $(BOLD)Cleaning build artifacts...$(SGR0)"
	@$(GRADLEW) clean

clean-all: clean ## Deep clean including caches
	@echo "🧹 $(BOLD)Wiping caches...$(SGR0)"
	@rm -rf .gradle
	@echo "$(GREEN)✅ Clean complete$(SGR0)"

# --- DESKTOP APPLICATION ---

run: check-tools ## Run the desktop application
	@echo "🚀 $(BOLD)Running application...$(SGR0)"
	@$(GRADLEW) $(APP_MODULE):run

dev: run ## Alias for run

# --- ANDROID ---

android-build: ## Build Android application (debug)
	@$(GRADLEW) $(ANDROID_APP):assembleDebug

android-lint: ## Run Android lint
	@$(GRADLEW) $(ANDROID_APP):lint

# --- RUST AGENT RUNTIME ---

rust-check: ## Run cargo check for agent runtime
	@$(GRADLEW) $(RUST_MODULE):cargoCheck -PenableRustTasks=true

rust-test: ## Run cargo tests for agent runtime
	@$(GRADLEW) $(RUST_MODULE):cargoTest -PenableRustTasks=true

rust-clippy: ## Run clippy for agent runtime
	@$(GRADLEW) $(RUST_MODULE):cargoClippy -PenableRustTasks=true

rust-fmt: ## Check Rust formatting
	@$(GRADLEW) $(RUST_MODULE):cargoFmtCheck -PenableRustTasks=true

rust-build: ## Build agent runtime binary
	@$(GRADLEW) $(RUST_MODULE):cargoBuild -PenableRustTasks=true

# --- WEB APPLICATIONS ---

web-install: ## Install web workspace dependencies
	@$(GRADLEW) $(WEB_MODULE):workspaceInstall

# Docs site
docs-dev: ## Run Docs dev server
	@$(GRADLEW) $(WEB_MODULE):docsDev
docs-build: ## Build Docs site
	@$(GRADLEW) $(WEB_MODULE):docsBuild
docs-check: ## Lint/Format check Docs (Biome)
	@$(GRADLEW) $(WEB_MODULE):docsCheck
docs-format: ## Format Docs (Biome)
	@$(GRADLEW) $(WEB_MODULE):docsFormat

# Chat app
chat-dev: ## Run Chat app dev server
	@$(GRADLEW) $(WEB_MODULE):chatDev
chat-build: ## Build Chat app
	@$(GRADLEW) $(WEB_MODULE):chatBuild
chat-check: ## Check Chat app
	@$(GRADLEW) $(WEB_MODULE):chatCheck
chat-test: ## Run Chat app tests
	@$(GRADLEW) $(WEB_MODULE):chatTestCoverage

# Dashboard
dashboard-dev: ## Run Dashboard dev server
	@$(GRADLEW) $(WEB_MODULE):dashboardDev
dashboard-build: ## Build Dashboard app
	@$(GRADLEW) $(WEB_MODULE):dashboardBuild
dashboard-check: ## Check Dashboard app
	@$(GRADLEW) $(WEB_MODULE):dashboardCheck
dashboard-test: ## Run Dashboard app tests
	@$(GRADLEW) $(WEB_MODULE):dashboardTestCoverage

# Marketing
marketing-dev: ## Run Marketing site dev server
	@$(GRADLEW) $(WEB_MODULE):marketingDev
marketing-build: ## Build Marketing site
	@$(GRADLEW) $(WEB_MODULE):marketingBuild
marketing-check: ## Check Marketing site
	@$(GRADLEW) $(WEB_MODULE):marketingCheck

web-build-all: ## Build all web applications
	@$(GRADLEW) $(WEB_MODULE):buildAllWebApps

web-clean-all: ## Clean all web applications
	@$(GRADLEW) $(WEB_MODULE):cleanAllWebApps

web-test-all: ## Run all web application tests
	@$(GRADLEW) $(WEB_MODULE):testCoverageAllWebApps

# --- QUALITY & LINTING ---

format: ## Apply formatting (Spotless)
	@$(GRADLEW) spotlessApply

check-format: ## Check code formatting without fixing
	@$(GRADLEW) spotlessCheck

check: ## Run all quality checks (lint, tests, etc)
	@$(GRADLEW) check -PenableRustTasks=true

lint-kotlin: ## Run Kotlin static analysis
	@$(GRADLEW) qualityCheck

lint-rust: ## Run Rust clippy
	@$(GRADLEW) $(RUST_MODULE):cargoClippy -PenableRustTasks=true

lint-android: ## Run Android lint
	@$(GRADLEW) $(ANDROID_APP):lint

lint-all: lint-kotlin lint-rust lint-android ## Run all linters

# --- TESTING ---

test: ## Run all tests
	@$(GRADLEW) test

test-app: ## Run tests for desktop app
	@$(GRADLEW) $(APP_MODULE):jvmTest

test-core: ## Run tests for core module
	@$(GRADLEW) $(CORE_MODULE):jvmTest

test-verbose: ## Run tests with verbose output
	@$(GRADLEW) test --info

test-coverage: ## Run tests with Kover coverage report
	@$(GRADLEW) koverHtmlReport
	@echo "📊 Report: $(APP_MODULE)/build/reports/kover/html/index.html"

# --- DOCUMENTATION ---

docs-code: ## Generate Kotlin documentation (Dokka)
	@$(GRADLEW) dokkaHtml

# --- DEPENDENCY MANAGEMENT ---

deps: ## Show project dependencies
	@$(GRADLEW) dependencies

deps-app: ## Show app module dependencies
	@$(GRADLEW) $(APP_MODULE):dependencies

deps-analysis: ## Run dependency analysis
	@$(GRADLEW) buildHealth

deps-update: ## Check for dependency updates
	@$(GRADLEW) dependencyUpdates

# --- DEV ENVIRONMENT (Docker) ---

dev-up: ## Start Docker dev environment
	@./dev/cli.sh up
dev-down: ## Stop Docker dev environment
	@docker compose -f dev/docker-compose.yml down
dev-shell: ## Enter Sandbox container
	@./dev/cli.sh shell
dev-agent: ## Enter Agent container
	@./dev/cli.sh agent
dev-logs: ## Follow Docker logs
	@docker compose -f dev/docker-compose.yml logs -f
dev-status: ## Show dev container status
	@docker compose -f dev/docker-compose.yml ps
dev-build: ## Rebuild dev images
	@./dev/cli.sh build
dev-clean: ## Stop and wipe dev environment
	@./dev/cli.sh clean

# --- LOCAL RUNTIME (Docker Compose) ---

runtime-up: ## Start local gateway runtime (clients/agent-runtime)
	@docker compose -f clients/agent-runtime/docker-compose.yml up -d
runtime-up-dashboard: ## Start local gateway + dashboard runtime
	@docker compose -f clients/agent-runtime/docker-compose.yml --profile dashboard up -d
runtime-down: ## Stop local gateway/dashboard runtime
	@docker compose -f clients/agent-runtime/docker-compose.yml down
runtime-logs: ## Follow local gateway/dashboard logs
	@docker compose -f clients/agent-runtime/docker-compose.yml logs -f
runtime-status: ## Show local gateway/dashboard status
	@docker compose -f clients/agent-runtime/docker-compose.yml ps

# --- CONTINUOUS INTEGRATION ---

ci-build: ## CI: Build without daemon
	@$(GRADLEW) build --no-daemon

ci-test: ## CI: Run tests without daemon
	@$(GRADLEW) test --no-daemon

ci-check: ## CI: Run all checks without daemon
	@$(GRADLEW) check -PenableRustTasks=true --no-daemon

# --- FULL WORKFLOWS ---

all: clean build check ## Run full pipeline (clean, build, check)
	@echo "$(GREEN)✨ Full pipeline completed!$(SGR0)"

quick: format build-fast ## Quick cycle (format + build-fast)
	@echo "$(GREEN)✨ Quick build completed!$(SGR0)"

# --- UTILITIES ---

tasks: ## List all available Gradle tasks
	@$(GRADLEW) tasks

info: ## Show project information
	@echo "$(BOLD)📋 Project Information:$(SGR0)"
	@echo "   OS: $(DETECTED_OS)"
	@$(GRADLEW) --version

version: ## Show project version
	@$(GRADLEW) --quiet version 2>/dev/null || echo "Run './gradlew version' for version info"

sync-version: ## Sync VERSION with git tag
	@bash ./sync-version-with-tag.sh

.PHONY: help h check-tools setup sync-agents wrapper build build-fast clean clean-all run dev \
        android-build android-lint rust-check rust-test rust-clippy rust-fmt rust-build \
        web-install docs-dev docs-build docs-check docs-format \
        chat-dev chat-build chat-check chat-test dashboard-dev dashboard-build dashboard-check dashboard-test \
        marketing-dev marketing-build marketing-check web-build-all web-clean-all web-test-all \
        format check-format check lint-kotlin lint-rust lint-android lint-all \
        test test-app test-core test-verbose test-coverage docs-code \
        deps deps-app deps-analysis deps-update \
         dev-up dev-down dev-shell dev-agent dev-logs dev-status dev-build dev-clean \
         runtime-up runtime-up-dashboard runtime-down runtime-logs runtime-status \
         ci-build ci-test ci-check all quick tasks info version sync-version
