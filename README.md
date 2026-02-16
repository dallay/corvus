# Corvus

Corvus is a reactive, always-on agent platform built from this Gradle baseline and customized for
long-running orchestration workloads.

## Core Architecture

- **Orchestrator**: Kotlin + Spring Boot + Coroutines/WebFlux.
- **Memory**: Graph-first knowledge model with Neo4j.
- **High-performance tools**: Rust sidecars for scraping and sandboxed execution.
- **Control plane**: Astro + Vue documentation and operator UI.

## Quick Commands

```bash
make setup          # Initial setup
make run            # Run app module
make build          # Full build with tests
make build-fast     # Build without tests
make test           # Run tests
make check          # Run format + lint + tests
```

## Repository

- GitHub: <https://github.com/dallay/corvus>
- Documentation source: `apps/docs/website/`

## Current Baseline

The project keeps the original Gradle build-logic and plugin namespace to preserve stability while
the product architecture evolves module by module.
