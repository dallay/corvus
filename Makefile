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
    # Ensure bash is present before running bash scripts on Windows.
    REQUIRE_BASH = $(if $(shell where bash 2>nul),,$(error Bash not found in PATH on Windows. Install Git Bash or enable WSL, then rerun make bootstrap-bash))
    SHELL := bash
else
    DETECTED_OS := $(shell uname -s 2>/dev/null || echo Unknown)
    SHELL := /bin/bash
    REQUIRE_BASH =
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

bootstrap-bash: ## Ensure bash is available (Windows)
	$(REQUIRE_BASH)
	@bash scripts/bootstrap-bash.sh


check-tools: bootstrap-bash ## Verify toolchain (Java 21, Node 22, pnpm 10, Rust 1.75)
	$(REQUIRE_BASH)
	@bash scripts/check-tools.sh

setup: check-tools ## Initial project setup (agents, web deps, rust check)
	@echo "🔧 $(BOLD)Setting up project...$(SGR0)"
	@chmod +x gradlew
	@$(GRADLEW) agentsyncApply
	@$(GRADLEW) $(WEB_MODULE):workspaceInstall
	@$(GRADLEW) $(RUST_MODULE):cargoCheck -PenableRustTasks=true
	@echo "$(GREEN)✅ Project setup complete!$(SGR0)"

doctor: bootstrap-bash ## Diagnose dev environment and repo health
	$(REQUIRE_BASH)
	@bash scripts/doctor.sh

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
	@echo "CLEAN: $(BOLD)Cleaning build artifacts...$(SGR0)"
	@$(GRADLEW) clean

clean-all: clean ## Deep clean including caches
	@echo "CLEAN: $(BOLD)Wiping caches...$(SGR0)"
	@rm -rf .gradle
	@echo "$(GREEN)✅ Clean complete$(SGR0)"

# --- DESKTOP APPLICATION ---

run: check-tools ## Run the desktop application
	@echo "🚀 $(BOLD)Running application...$(SGR0)"
	@$(GRADLEW) $(APP_MODULE):run

dev: setup run ## Setup + run (recommended local dev)

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

rust-test-matrix: ## Run bootstrap feature-flag matrix for agent runtime
	@cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib bootstrap_feature_flag_matrix_reports_expected_assembly
	@cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib --no-default-features bootstrap_feature_flag_matrix_reports_expected_assembly
	@cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib --no-default-features --features mcp-runtime bootstrap_feature_flag_matrix_reports_expected_assembly
	@cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib --no-default-features --features hardware bootstrap_feature_flag_matrix_reports_expected_assembly
	@cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib --no-default-features --features memory-surreal bootstrap_feature_flag_matrix_reports_expected_assembly
	@cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib --no-default-features --features "mcp-runtime,memory-surreal,hardware" bootstrap_feature_flag_matrix_reports_expected_assembly

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

web-check-all: ## Run all web application checks
	@$(GRADLEW) $(WEB_MODULE):docsCheck
	@$(GRADLEW) $(WEB_MODULE):chatCheck
	@$(GRADLEW) $(WEB_MODULE):dashboardCheck
	@$(GRADLEW) $(WEB_MODULE):marketingCheck

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

test-coverage: rust-coverage ## Run coverage reports
	@$(GRADLEW) :agent-core-kmp:koverHtmlReport
	@echo "📊 Kotlin report: modules/agent-core-kmp/build/reports/kover/html/index.html"
	@echo "📊 Rust report: coverage/agent-runtime-coverage.lcov"

rust-coverage: ## Run Rust coverage for agent-runtime
	@command -v cargo-llvm-cov >/dev/null 2>&1 || { \
		echo "cargo-llvm-cov is required. Install with: cargo install cargo-llvm-cov" >&2; \
		exit 1; \
	}
	@rustup component list --installed | grep -Eq '^llvm-tools-preview|^llvm-tools' || { \
		echo "llvm-tools-preview (or llvm-tools) is required. Install with: rustup component add llvm-tools-preview" >&2; \
		exit 1; \
	}
	@mkdir -p coverage
	@cd clients/agent-runtime && cargo llvm-cov --lcov --output-path ../../coverage/agent-runtime-coverage.lcov

test-all: test rust-test web-test-all ## Run all tests (Gradle + Rust + Web)

check-all: check-format lint-all web-check-all test-all ## Full quality gate

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

dev-up: ## Start proxied dev environment at corvus.localhost
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

clean-web: ## Clean web app build outputs
	@$(GRADLEW) $(WEB_MODULE):cleanAllWebApps

clean-pnpm: ## Clean pnpm store (optional)
	@if command -v pnpm >/dev/null 2>&1; then pnpm store prune; else echo "ℹ️  pnpm not installed"; fi

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

all: clean build check-all ## Run full pipeline (clean, build, check-all)
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
	$(REQUIRE_BASH)
	@bash ./scripts/sync-version-with-tag.sh

.PHONY: help h check-tools setup doctor sync-agents wrapper build build-fast clean clean-all run dev \
        android-build android-lint rust-check rust-test rust-clippy rust-fmt rust-build \
        web-install docs-dev docs-build docs-check docs-format \
        chat-dev chat-build chat-check chat-test dashboard-dev dashboard-build dashboard-check dashboard-test \
        marketing-dev marketing-build marketing-check web-build-all web-clean-all web-test-all web-check-all \
        format check-format check lint-kotlin lint-rust lint-android lint-all \
        test test-app test-core test-verbose test-coverage rust-coverage test-all check-all docs-code \
        deps deps-app deps-analysis deps-update \
         dev-up dev-down dev-shell dev-agent dev-logs dev-status dev-build dev-clean clean-web clean-pnpm \
         runtime-up runtime-up-dashboard runtime-down runtime-logs runtime-status \
         ci-build ci-test ci-check all quick tasks info version sync-version
