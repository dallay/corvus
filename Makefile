# ====================================================================================
# STARTER-GRADLE MAKEFILE
#
# Standardized commands for all developers and operating systems.
# Run `make help` to see all available commands.
# ====================================================================================

.DEFAULT_GOAL := help

MAKEFLAGS += --no-builtin-rules --no-builtin-variables --output-sync=target
.SHELLFLAGS := -eu -o pipefail -c

# ------------------------------------------------------------------------------------
# VARIABLES & CONFIGURATION
# ------------------------------------------------------------------------------------

# Operating System Detection & Shell Normalization
ifeq ($(OS),Windows_NT)
    DETECTED_OS := Windows
    # Find bash.exe and convert to short path (8.3) to avoid space issues
    SHELL_PATH := $(shell for /f "delims=" %i in ('where bash.exe 2^>NUL') do @(for %j in ("%i") do @echo %~sj & exit /b 0))
    ifeq ($(SHELL_PATH),)
        $(error ❌ A bash-compatible shell (Git Bash, WSL) is required on Windows. See README.md)
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
CHECK_TOOLS ?= 1

# Module Names
APP_MODULE := composeApp
DOCS_MODULE := docs

# ------------------------------------------------------------------------------------
# CORE & HELP
# ------------------------------------------------------------------------------------

help: ## Show this help message
	@echo "╔═══════════════════════════════════════════════════════════════════════╗"
	@echo "║              STARTER-GRADLE - AVAILABLE COMMANDS                      ║"
	@echo "╚═══════════════════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "$(shell tput bold)Quick Start:$(shell tput sgr0)"
	@echo "  make run           - Run the main application"
	@echo "  make build         - Build the entire project"
	@echo "  make test          - Run all tests"
	@echo "  make check         - Run all checks (format, lint, tests)"
	@echo ""
	@echo "$(shell tput bold)Targets:$(shell tput sgr0)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z0-9_-]+:.*?## / {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# ------------------------------------------------------------------------------------
# ENVIRONMENT & SETUP
# ------------------------------------------------------------------------------------

check-tools: ## Verify required tools are installed
ifeq ($(CHECK_TOOLS),0)
	@echo "ℹ️  Skipping toolchain checks (CHECK_TOOLS=0)"
else
	@echo "🔍 Checking required tools and versions..."
	@bash -ec '\
		require_cmd() { \
			command -v "$$1" >/dev/null 2>&1 || { echo "❌ Error: '\''$$1'\'' is not installed."; exit 1; }; \
		}; \
		require_cmd java; \
		require_cmd git; \
		require_cmd node; \
		require_cmd pnpm; \
		require_cmd rustc; \
		require_cmd cargo; \
		java_major=$$(java -version 2>&1 | sed -nE '\''s/.*version "([0-9]+).*/\1/p'\'' | head -n1); \
		node_major=$$(node -p "process.versions.node.split('\''.'\'')[0]"); \
		pnpm_major=$$(pnpm --version | awk -F. '\''{print $$1}'\''); \
		rust_ver=$$(rustc --version | awk '\''{print $$2}'\''); \
		rust_major=$${rust_ver%%.*}; \
		rust_minor_part=$${rust_ver#*.}; \
		rust_minor=$${rust_minor_part%%.*}; \
		if [ -z "$$java_major" ] || [ "$$java_major" -lt 21 ]; then \
			echo "❌ Error: JDK 21+ required. Current java major: $${java_major:-unknown}"; \
			exit 1; \
		fi; \
		if [ "$$node_major" -lt 22 ]; then \
			echo "❌ Error: Node.js 22+ required. Current Node major: $$node_major"; \
			exit 1; \
		fi; \
		if [ "$$pnpm_major" -lt 10 ]; then \
			echo "❌ Error: pnpm 10+ required. Current pnpm major: $$pnpm_major"; \
			exit 1; \
		fi; \
		if [ "$$rust_major" -lt 1 ] || { [ "$$rust_major" -eq 1 ] && [ "$$rust_minor" -lt 75 ]; }; then \
			echo "❌ Error: Rust 1.75+ required. Current rustc: $$rust_ver"; \
			exit 1; \
		fi; \
		if ! command -v docker >/dev/null 2>&1; then \
			echo "⚠️  Docker is not installed (optional; required for sandbox/dev containers)."; \
		fi; \
		if [ "$$(uname -s 2>/dev/null || echo unknown)" = "Darwin" ]; then \
			if ! command -v xcodebuild >/dev/null 2>&1; then \
				echo "⚠️  Xcode CLI tools not found (optional; required for iOS development)."; \
			fi; \
		fi; \
		echo "✅ Toolchain OK: java=$$java_major, node=$$node_major, pnpm=$$pnpm_major, rustc=$$rust_ver"; \
	'
endif

setup: check-tools ## Initial project setup (chmod +x gradlew)
	@echo "🔧 Setting up project..."
	@chmod +x gradlew
	@git update-index --chmod=+x gradlew || \
		echo "⚠️  Could not update git index permissions for gradlew (continuing)."
	@echo "📦 Initializing Gradle wrapper..."
	@GRADLE_USER_HOME=$${GRADLE_USER_HOME:-$(CURDIR)/.gradle} $(GRADLEW) --version >/dev/null
	@echo "🤖 Synchronizing AI agents..."
	@GRADLE_USER_HOME=$${GRADLE_USER_HOME:-$(CURDIR)/.gradle} $(GRADLEW) agentsyncApply
	@echo "📦 Installing web workspace dependencies..."
	@GRADLE_USER_HOME=$${GRADLE_USER_HOME:-$(CURDIR)/.gradle} $(GRADLEW) :web:workspaceInstall
	@echo "🦀 Validating Rust workspace (agent runtime)..."
	@GRADLE_USER_HOME=$${GRADLE_USER_HOME:-$(CURDIR)/.gradle} $(GRADLEW) :agent-runtime:cargoCheck
	@echo "✅ Project setup complete: tools validated, agents synced, web deps installed, Rust checked"

sync-agents: check-tools ## Synchronize AI agent configurations (agentsync)
	@echo "🤖 Synchronizing AI agents..."
	@GRADLE_USER_HOME=$${GRADLE_USER_HOME:-$(CURDIR)/.gradle} $(GRADLEW) agentsyncApply

wrapper: ## Update Gradle wrapper
	@$(GRADLEW) wrapper --gradle-version $(shell grep -E '^gradle\s*=' gradle/libs.versions.toml | sed 's/.*= "\(.*\)".*/\1/')

# ------------------------------------------------------------------------------------
# BUILD
# ------------------------------------------------------------------------------------

build: check-tools ## Build the entire project
	@echo "🏗️  Building project..."
	@$(GRADLEW) build

build-fast: check-tools ## Build without running tests (faster)
	@echo "🏗️  Building project (skip tests)..."
	@$(GRADLEW) build -x test

clean: ## Clean build artifacts
	@echo "🧹 Cleaning build artifacts..."
	@$(GRADLEW) clean

clean-all: clean ## Clean everything including Gradle caches
	@echo "🧹 Cleaning Gradle caches..."
	@rm -rf .gradle
	@echo "✅ All artifacts cleaned"

# ------------------------------------------------------------------------------------
# DEVELOPMENT
# ------------------------------------------------------------------------------------

run: check-tools ## Run the main application (compose desktop module)
	@echo "🚀 Running application..."
	@$(GRADLEW) $(APP_MODULE):run

dev: run ## Alias for 'make run'

# ------------------------------------------------------------------------------------
# DEV ENVIRONMENT (Docker)
# ------------------------------------------------------------------------------------

dev-up: ## Start dev environment (Agent + Sandbox containers)
	@echo "🚀 Starting Dev Environment..."
	@./dev/cli.sh up

dev-down: ## Stop dev containers
	@echo "🛑 Stopping dev containers..."
	@docker compose -f dev/docker-compose.yml down

dev-shell: ## Enter Sandbox (Ubuntu) - simulate user environment
	@echo "💻 Entering Sandbox..."
	@./dev/cli.sh shell

dev-agent: ## Enter Agent container (Corvus CLI) - debug the binary
	@echo "🤖 Entering Agent container..."
	@./dev/cli.sh agent

dev-logs: ## View dev container logs (follow mode)
	@echo "📜 Following logs..."
	@docker compose -f dev/docker-compose.yml logs -f

dev-build: ## Rebuild dev images and restart
	@echo "🔨 Rebuilding dev images..."
	@./dev/cli.sh build

dev-clean: ## Stop containers and wipe workspace data
	@echo "⚠️  Cleaning dev environment..."
	@./dev/cli.sh clean

dev-status: ## Show dev container status
	@docker compose -f dev/docker-compose.yml ps

# ------------------------------------------------------------------------------------
# TESTING
# ------------------------------------------------------------------------------------

test: check-tools ## Run all tests
	@echo "🧪 Running all tests..."
	@$(GRADLEW) test

test-app: check-tools ## Run tests for app module only
	@echo "🧪 Running app tests..."
	@$(GRADLEW) $(APP_MODULE):jvmTest

test-coverage: check-tools ## Run tests with coverage report (Kover)
	@echo "🧪 Running tests with coverage..."
	@$(GRADLEW) koverHtmlReport
	@echo "📊 Coverage report: $(APP_MODULE)/build/reports/kover/html/index.html"

test-verbose: check-tools ## Run tests with verbose output
	@echo "🧪 Running tests (verbose)..."
	@$(GRADLEW) test --info

# ------------------------------------------------------------------------------------
# CODE QUALITY & FORMATTING
# ------------------------------------------------------------------------------------

format: check-tools ## Format all code (Spotless)
	@echo "✨ Formatting code..."
	@$(GRADLEW) spotlessApply

check-format: check-tools ## Check code formatting without fixing
	@echo "🔍 Checking code formatting..."
	@$(GRADLEW) spotlessCheck

lint-kotlin: check-tools ## Run Kotlin static analysis (via qualityCheck)
	@echo "🔍 Running Kotlin static analysis (qualityCheck)..."
	@$(GRADLEW) qualityCheck

lint-java: check-tools ## Run Java static analysis (via qualityCheck)
	@echo "🔍 Running Java static analysis (qualityCheck)..."
	@$(GRADLEW) qualityCheck

lint-rust: check-tools ## Run Rust static analysis (Clippy)
	@echo "🔍 Running Rust static analysis (Clippy)..."
	@$(GRADLEW) :agent-runtime:cargoClippy -PenableRustTasks=true

lint-android: check-tools ## Run Android static analysis (Lint)
	@echo "🔍 Running Android static analysis (Lint)..."
	@$(GRADLEW) :androidApp:lint

lint-all: lint-kotlin lint-java lint-rust lint-android ## Run all static analysis

check: check-tools ## Run all checks (format, lint, tests, Rust, Android)
	@echo "🔍 Running Gradle check (includes tests, format, quality checks)..."
	@$(GRADLEW) check -PenableRustTasks=true
	@echo ""
	@echo "🔍 Running Android Lint..."
	@$(GRADLEW) :androidApp:lint

# ------------------------------------------------------------------------------------
# DOCUMENTATION
# ------------------------------------------------------------------------------------

docs: check-tools ## Generate documentation (Dokka)
	@echo "📚 Generating documentation..."
	@$(GRADLEW) dokkaHtml

docs-serve: docs ## Generate and serve documentation locally
	@echo "📚 Documentation generated in: build/dokka/html/"
	@echo "📖 Open the index.html file in your browser"

docs-web-build: check-tools ## Build website docs (Astro/Starlight)
	@echo "🌐 Building website docs..."
	@$(GRADLEW) :$(DOCS_MODULE):docStarlight

docs-web-check: check-tools ## Check website docs formatting/lint (Biome)
	@echo "🔎 Checking website docs..."
	@$(GRADLEW) :$(DOCS_MODULE):websiteCheck

docs-web-format: check-tools ## Format website docs (Biome)
	@echo "✨ Formatting website docs..."
	@$(GRADLEW) :$(DOCS_MODULE):websiteFormat

docs-web-dev: check-tools ## Run website docs dev server
	@echo "🌐 Starting docs dev server..."
	@cd $(DOCS_MODULE)/website && pnpm run dev

# ------------------------------------------------------------------------------------
# DEPENDENCY MANAGEMENT
# ------------------------------------------------------------------------------------

deps: check-tools ## Show project dependencies
	@echo "📦 Project dependencies:"
	@$(GRADLEW) dependencies

deps-app: check-tools ## Show app module dependencies
	@echo "📦 App module dependencies:"
	@$(GRADLEW) $(APP_MODULE):dependencies

deps-analysis: check-tools ## Run dependency analysis
	@echo "🔍 Analyzing dependencies..."
	@$(GRADLEW) buildHealth

deps-update: check-tools ## Check for dependency updates
	@echo "🔄 Checking for updates..."
	@$(GRADLEW) dependencyUpdates

# ------------------------------------------------------------------------------------
# UTILITY
# ------------------------------------------------------------------------------------

tasks: check-tools ## List all available Gradle tasks
	@$(GRADLEW) tasks

info: check-tools ## Show project information
	@echo "📋 Project Information:"
	@echo "   OS: $(DETECTED_OS)"
	@echo "   Shell: $(SHELL)"
	@$(GRADLEW) --version

version: check-tools ## Show project version
	@$(GRADLEW) --quiet version 2>/dev/null || echo "Run './gradlew version' for version info"

# ------------------------------------------------------------------------------------
# CONTINUOUS INTEGRATION
# ------------------------------------------------------------------------------------

ci-build: check-tools ## CI: Build without daemon
	@$(GRADLEW) build --no-daemon

ci-test: check-tools ## CI: Run tests without daemon
	@$(GRADLEW) test --no-daemon

ci-check: check-tools ## CI: Run all checks without daemon
	@$(GRADLEW) check -PenableRustTasks=true --no-daemon

# ------------------------------------------------------------------------------------
# FULL WORKFLOWS
# ------------------------------------------------------------------------------------

all: check-tools ## Fast-fail quality pipeline (parallel)
	@echo "🚦 Running fast-fail pipeline (parallel): format, tests, quality, rust"
	@python3 dev/make-fast-fail.py "$(MAKE)" check-format test lint-kotlin lint-rust

all-full: clean build check ## Full CI pipeline (sequential)
	@echo "✨ Full CI pipeline completed successfully!"

quick: format build-fast ## Quick development cycle (format + build without tests)
	@echo "✨ Quick build completed!"

sync-version: ## Sync VERSION in gradle.properties with the latest git tag (vX.Y.Z)
	@bash ./sync-version-with-tag.sh

.PHONY: help check-tools setup wrapper build build-fast clean clean-all run dev \
        dev-up dev-down dev-shell dev-agent dev-logs dev-build dev-clean dev-status \
        test test-app test-coverage test-verbose \
        format check-format lint-kotlin lint-java lint-rust lint-android lint-all check docs docs-serve \
        docs-web-build docs-web-check docs-web-format docs-web-dev \
        deps deps-app deps-analysis deps-update tasks info version ci-build \
        ci-test ci-check all all-full quick sync-version
