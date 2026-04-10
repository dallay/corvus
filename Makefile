# ====================================================================================
# CORVUS MONOREPO - MAKEFILE
#
# Standardized commands for all developers and operating systems.
# On Windows, run them from Git Bash (or WSL) so the same make targets work unchanged.
# Run `make help` to see all available commands.
# ====================================================================================

.DEFAULT_GOAL := help

# ------------------------------------------------------------------------------------
# VARIABLES & CONFIGURATION
# ------------------------------------------------------------------------------------

# Operating System Detection & Shell Normalization
ifeq ($(OS),Windows_NT)
    DETECTED_OS := Windows
    SHELL := bash
else
    DETECTED_OS := $(shell uname -s 2>/dev/null || echo Unknown)
    SHELL := /bin/bash
endif

# Common Constants
GRADLEW := bash ./scripts/gradlew.sh
DEV_CLI := bash ./dev/cli.sh
RUNTIME_CLI := bash ./scripts/runtime-compose.sh
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
	@bash ./scripts/print-make-help.sh $(MAKEFILE_LIST)

h: help ## Alias for help

# --- ENVIRONMENT & SETUP ---

bootstrap-bash: ## Ensure bash is available (Windows)
	@bash scripts/bootstrap-bash.sh


check-tools: bootstrap-bash ## Verify toolchain (Java 21, Node 22, pnpm 10, Rust 1.75)
	@bash scripts/check-tools.sh

install-hooks: ## Install repository git hooks
	@bash scripts/install-hooks.sh

setup: check-tools install-hooks ## Initial project setup (hooks, agents, web deps, rust check)
	@echo "🔧 $(BOLD)Setting up project...$(SGR0)"
	@$(GRADLEW) agentsyncApply
	@$(GRADLEW) $(WEB_MODULE):workspaceInstall
	@$(GRADLEW) $(RUST_MODULE):cargoCheck -PenableRustTasks=true
	@echo "$(GREEN)✅ Project setup complete!$(SGR0)"

doctor: bootstrap-bash ## Diagnose dev environment and repo health
	@bash scripts/doctor.sh
sync-agents: ## Sync AI agent configurations (agentsync)
	@$(GRADLEW) agentsyncApply

wrapper: ## Update Gradle wrapper
	@$(GRADLEW) wrapper --gradle-version "$$(node -e "const fs=require('fs'); const m=fs.readFileSync('gradle/libs.versions.toml','utf8').match(/^gradle\s*=\s*\"([^\"]+)\"/m); if(!m){process.exit(1)} process.stdout.write(m[1])")"

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
	@echo "$(GREEN)✅ Clean complete$(SGR0)"

clean-all: ## Clean everything: Gradle + Rust + web + pnpm + caches
	@echo "CLEAN: $(BOLD)Cleaning all build artifacts...$(SGR0)"
	@$(GRADLEW) clean
	@echo "CLEAN: $(BOLD)Cleaning Rust targets...$(SGR0)"
	@ cargo clean --manifest-path clients/agent-runtime/Cargo.toml
	@echo "CLEAN: $(BOLD)Cleaning web outputs...$(SGR0)"
	@$(GRADLEW) :cleanAllWebApps
	@echo "CLEAN: $(BOLD)Cleaning pnpm store...$(SGR0)"
	@if command -v pnpm >/dev/null 2>&1; then pnpm store prune; fi
	@echo "CLEAN: $(BOLD)Wiping Gradle caches...$(SGR0)"
	@rm -rf .gradle
	@echo "$(GREEN)✅ Full clean complete$(SGR0)"

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

# --- CEREBRO MEMORY SERVICE ---

cerebro-check: ## Run cargo check for cerebro
	@cargo check --manifest-path modules/cerebro/Cargo.toml

cerebro-test: ## Run cargo tests for cerebro
	@cargo test --manifest-path modules/cerebro/Cargo.toml

cerebro-clippy: ## Run clippy for cerebro
	@cargo clippy --manifest-path modules/cerebro/Cargo.toml --all-targets -- -D warnings

cerebro-fmt: ## Check Rust formatting for cerebro
	@cargo fmt --manifest-path modules/cerebro/Cargo.toml -- --check

cerebro-build: ## Build cerebro binary
	@cargo build --manifest-path modules/cerebro/Cargo.toml --release --bin cerebro

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

link-check: ## Check repository links with Lychee (local + remote)
	@command -v lychee >/dev/null 2>&1 || { \
		echo "lychee is required. Install from https://github.com/lycheeverse/lychee" >&2; \
		exit 1; \
	}
	@lychee --config lychee.toml --no-progress .

link-check-local: ## Check repository local links with Lychee (offline)
	@command -v lychee >/dev/null 2>&1 || { \
		echo "lychee is required. Install from https://github.com/lycheeverse/lychee" >&2; \
		exit 1; \
	}
	@lychee --config lychee.toml --offline --no-progress .

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
	@$(DEV_CLI) up
dev-up-dashboard: ## Start proxied dev environment with dashboard at corvus.localhost
	@$(DEV_CLI) up-dashboard
dev-down: ## Stop Docker dev environment
	@$(DEV_CLI) down
dev-shell: ## Enter Sandbox container
	@$(DEV_CLI) shell
dev-agent: ## Enter Agent container
	@$(DEV_CLI) agent
dev-logs: ## Follow Docker logs
	@$(DEV_CLI) logs
dev-status: ## Show dev container status
	@$(DEV_CLI) status
dev-build: ## Rebuild dev images
	@$(DEV_CLI) build
dev-clean: ## Stop and wipe dev environment
	@$(DEV_CLI) clean

clean-web: ## Clean web app build outputs
	@$(GRADLEW) $(WEB_MODULE):cleanAllWebApps

clean-pnpm: ## Clean pnpm store (optional)
	@if command -v pnpm >/dev/null 2>&1; then pnpm store prune; else echo "ℹ️  pnpm not installed"; fi

# --- LOCAL RUNTIME (Docker Compose) ---

runtime-up: ## Start local gateway runtime (clients/agent-runtime)
	@$(RUNTIME_CLI) up
runtime-up-dashboard: ## Start local gateway + dashboard runtime
	@$(RUNTIME_CLI) up-dashboard
runtime-down: ## Stop local gateway/dashboard runtime
	@$(RUNTIME_CLI) down
runtime-logs: ## Follow local gateway/dashboard logs
	@$(RUNTIME_CLI) logs
runtime-status: ## Show local gateway/dashboard status
	@$(RUNTIME_CLI) status

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
	@bash ./scripts/sync-version-with-tag.sh

.PHONY: help h check-tools setup doctor sync-agents wrapper build build-fast clean clean-all run dev \
        android-build android-lint rust-check rust-test rust-clippy rust-fmt rust-build \
        cerebro-check cerebro-test cerebro-clippy cerebro-fmt cerebro-build \
        web-install docs-dev docs-build docs-check docs-format \
        dashboard-dev dashboard-build dashboard-check dashboard-test \
        marketing-dev marketing-build marketing-check web-build-all web-clean-all web-test-all web-check-all \
        format check-format check lint-kotlin lint-rust lint-android lint-all \
        test test-app test-core test-verbose test-coverage rust-coverage test-all check-all docs-code \
        deps deps-app deps-analysis deps-update \
        dev-up dev-up-dashboard dev-down dev-shell dev-agent dev-logs dev-status dev-build dev-clean clean-web clean-pnpm \
        runtime-up runtime-up-dashboard runtime-down runtime-logs runtime-status \
        ci-build ci-test ci-check all quick tasks info version sync-version
