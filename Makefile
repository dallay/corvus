# ====================================================================================
# STARTER-GRADLE MAKEFILE
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

# Module Names
APP_MODULE := app
DOCS_MODULE := docs
EXAMPLE_JAVA := example-java
EXAMPLE_KOTLIN := example-kotlin
EXAMPLE_SPRING := example-spring

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
	@echo "🔍 Checking required tools..."
	@command -v java >/dev/null 2>&1 || { echo "❌ Error: 'java' is not installed. Please install JDK 21+"; exit 1; }
	@command -v git >/dev/null 2>&1 || { echo "❌ Error: 'git' is not installed."; exit 1; }
	@echo "✅ All required tools are available"

setup: check-tools ## Initial project setup (chmod +x gradlew)
	@echo "🔧 Setting up project..."
	@chmod +x gradlew
	@git update-index --chmod=+x gradlew
	@$(GRADLEW) agentsyncApply || true
	@echo "✅ Project setup complete"

sync-agents: check-tools ## Synchronize AI agent configurations (agentsync)
	@echo "🤖 Synchronizing AI agents..."
	@$(GRADLEW) agentsyncApply

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

run: check-tools ## Run the main application (app module)
	@echo "🚀 Running application..."
	@$(GRADLEW) $(APP_MODULE):run

dev: run ## Alias for 'make run'

run-java: check-tools ## Run the Java example
	@echo "🚀 Running Java example..."
	@$(GRADLEW) $(EXAMPLE_JAVA):run

run-kotlin: check-tools ## Run the Kotlin example
	@echo "🚀 Running Kotlin example..."
	@$(GRADLEW) $(EXAMPLE_KOTLIN):run

run-spring: check-tools ## Run the Spring example
	@echo "🚀 Running Spring example..."
	@$(GRADLEW) $(EXAMPLE_SPRING):bootRun

# ------------------------------------------------------------------------------------
# TESTING
# ------------------------------------------------------------------------------------

test: check-tools ## Run all tests
	@echo "🧪 Running all tests..."
	@$(GRADLEW) test

test-app: check-tools ## Run tests for app module only
	@echo "🧪 Running app tests..."
	@$(GRADLEW) $(APP_MODULE):test

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

lint-kotlin: check-tools ## Run Kotlin linting (Detekt)
	@echo "🔍 Running Kotlin static analysis (Detekt)..."
	@$(GRADLEW) detekt

lint-java: check-tools ## Run Java static analysis (SpotBugs)
	@echo "🔍 Running Java static analysis (SpotBugs)..."
	@$(GRADLEW) spotbugsMain

lint: lint-kotlin lint-java ## Run all static analysis

check: check-tools ## Run all checks (format, lint, tests)
	@echo "🔍 Running all checks..."
	@$(GRADLEW) check

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
	@cd docs/website && pnpm run dev

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
	@$(GRADLEW) check --no-daemon

# ------------------------------------------------------------------------------------
# FULL WORKFLOWS
# ------------------------------------------------------------------------------------

all: clean build check ## Run full CI pipeline (clean, build, check)
	@echo "✨ Full CI pipeline completed successfully!"

quick: format build-fast ## Quick development cycle (format + build without tests)
	@echo "✨ Quick build completed!"

sync-version: ## Sync VERSION in gradle.properties with the latest git tag (vX.Y.Z)
	@bash ./sync-version-with-tag.sh

.PHONY: help check-tools setup wrapper build build-fast clean clean-all run dev \
        run-java run-kotlin run-spring test test-app test-coverage test-verbose \
        format check-format lint-kotlin lint-java lint check docs docs-serve \
        docs-web-build docs-web-check docs-web-format docs-web-dev \
        deps deps-app deps-analysis deps-update tasks info version ci-build \
        ci-test ci-check all quick sync-version
