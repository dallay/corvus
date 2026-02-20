# 🦅 Corvus

**A reactive, always-on agent platform for long-running orchestration workloads.**

Corvus is a highly extensible, multi-interface agentic platform designed to bridge the gap between AI autonomy and human supervision. Built with a robust Kotlin Multiplatform foundation and powered by a high-performance Rust runtime, Corvus provides a secure, sandboxed environment for AI agents to perform complex, multi-step tasks.

---

## 📖 Table of Contents

- [Features](#-features)
- [Tech Stack](#%EF%B8%8F-tech-stack)
- [Project Structure](#-project-structure)
- [Getting Started](#-getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
  - [Running the App](#running-the-app)
- [Development](#-development)
  - [Useful Commands](#useful-commands)
  - [Local Sandbox Environment](#local-sandbox-environment)
- [Documentation](#-documentation)
- [Contributing](#-contributing)
- [License](#-license)

---

## ✨ Features

- **Multi-Interface Support**: Interact with Corvus via CLI, a Compose Multiplatform Desktop app, or a web-based dashboard.
- **Always-On Autonomy**: A daemon mode for long-running agents that can handle background tasks and persistent orchestration.
- **Secure Sandboxing**: Execute dangerous commands safely within isolated Docker containers or restricted native runtimes.
- **Standardized Identity (AIEOS)**: Support for [AIEOS](https://aieos.org) v1.1, allowing for portable and model-agnostic AI personas.
- **Hybrid Memory Model**: Pluggable memory backends including SQLite, Neo4j, and SurrealDB for high-context retrieval.
- **Rich Integrations**: First-class support for WhatsApp (via Meta Cloud API), git, npm, cargo, and more.

---

## 🛠️ Tech Stack

- **Core Logic**: [Kotlin Multiplatform (KMP)](https://kotlinlang.org/docs/multiplatform.html)
- **Agent Runtime**: [Rust](https://www.rust-lang.org/) (High-performance sidecars and CLI)
- **Desktop UI**: [Compose Multiplatform](https://www.jetbrains.com/lp/compose-multiplatform/)
- **Web Stack**: [Astro](https://astro.build/), [Vue 3](https://vuejs.org/), and [Tailwind CSS](https://tailwindcss.com/)
- **Documentation**: [Starlight](https://starlight.astro.build/)
- **Build System**: [Gradle](https://gradle.org/) & [Makefile](https://www.gnu.org/software/make/)

---

## 📂 Project Structure

This repository is organized as a monorepo:

```text
corvus/
├── clients/
│   ├── agent-runtime/    # High-performance Rust Agent Core & CLI
│   ├── composeApp/       # Shared UI logic for Desktop/Mobile
│   ├── web/              # Web monorepo (Docs, Dashboard, Marketing)
│   ├── androidApp/       # Android specific wrapper
│   └── iosApp/           # iOS specific wrapper
├── modules/
│   └── agent-core-kmp/   # Core Kotlin Multiplatform logic & contracts
├── dev/                  # Local development environment (Docker/Sandbox)
├── gradle/               # Build logic and configurations
└── Makefile              # Standard entry point for development tasks
```

---

## 🚀 Getting Started

### Prerequisites

- **JDK 17+** (for Kotlin/KMP)
- **Rust 1.75+** (for Agent Runtime)
- **Node.js 22+** & **pnpm 10+** (for Web Apps)
- **Docker** (optional, for Sandboxing)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/dallay/corvus.git
   cd corvus
   ```
2. Run the initial setup:
   ```bash
   make setup
   ```

### Running the App

Start the Compose Multiplatform Desktop application:

```bash
make run
```

---

## 🛠 Development

### Useful Commands

We use a `Makefile` to standardize common operations:

| Command       | Description                              |
| ------------- | ---------------------------------------- |
| `make build`  | Full build with tests                    |
| `make test`   | Run all tests (Kotlin & Rust)            |
| `make format` | Apply code formatting (Spotless & Biome) |
| `make check`  | Run format, lint, and tests              |
| `make clean`  | Remove build artifacts                   |

### Local Sandbox Environment

To test agents in a controlled environment, you can spin up the local dev stack:

```bash
make dev-up      # Start Agent + Sandbox containers
make dev-shell   # Enter the Sandbox (Ubuntu)
make dev-down    # Stop the environment
```

---

## 📚 Documentation

Detailed documentation is available in the `clients/web/apps/docs/` directory. You can build and view it locally:

```bash
make docs-web-build
make docs-web-dev
```

---

## 🤝 Contributing

Contributions are welcome! Please read our `CONTRIBUTING.md` (if available) and ensure you run `make check` before submitting a Pull Request.

---

## 📄 License

This project is licensed under the Apache License 2.0. See the [LICENSE](LICENSE) file for details.

---

_(Note: Corvus is currently in active development. Features and architecture are subject to evolution.)_
