# 🐦‍⬛ Corvus

![corvus-ai.png](assets/corvus.png)

**A reactive, always-on agent platform for long-running orchestration workloads.**

<div align="center">

## 📊 Repository Stats

[![Stars](https://img.shields.io/github/stars/dallay/corvus?style=social)](https://github.com/dallay/corvus/stargazers)
[![Forks](https://img.shields.io/github/forks/dallay/corvus?style=social)](https://github.com/dallay/corvus/network/members)
[![Issues](https://img.shields.io/github/issues/dallay/corvus)](https://github.com/dallay/corvus/issues)
[![Pull Requests](https://img.shields.io/github/issues-pr/dallay/corvus)](https://github.com/dallay/corvus/pulls)
[![Repo Size](https://img.shields.io/github/repo-size/dallay/corvus)](https://github.com/dallay/corvus)
[![Last Commit](https://img.shields.io/github/last-commit/dallay/corvus)](https://github.com/dallay/corvus/commits/main)

## 🚀 Project Status

[![Build Status](https://github.com/dallay/corvus/actions/workflows/pull-request-check.yml/badge.svg)](https://github.com/dallay/corvus/actions/workflows/pull-request-check.yml)
[![codecov](https://codecov.io/gh/dallay/corvus/graph/badge.svg?token=N4THEP2OF1)](https://app.codecov.io/gh/dallay/corvus)
[![License](https://img.shields.io/github/license/dallay/corvus?color=blue)](LICENSE)
[![Version](https://img.shields.io/github/v/tag/dallay/corvus?sort=semver&label=version)](https://github.com/dallay/corvus/tags)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square)](https://github.com/dallay/corvus/compare)

## 🛡️ Code Quality (SonarCloud)

[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=dallay_corvus&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=dallay_corvus)
[![Bugs](https://sonarcloud.io/api/project_badges/measure?project=dallay_corvus&metric=bugs)](https://sonarcloud.io/summary/new_code?id=dallay_corvus)
[![Code Smells](https://sonarcloud.io/api/project_badges/measure?project=dallay_corvus&metric=code_smells)](https://sonarcloud.io/summary/new_code?id=dallay_corvus)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=dallay_corvus&metric=coverage)](https://sonarcloud.io/summary/new_code?id=dallay_corvus)
[![Vulnerabilities](https://sonarcloud.io/api/project_badges/measure?project=dallay_corvus&metric=vulnerabilities)](https://sonarcloud.io/summary/new_code?id=dallay_corvus)

## 🛠️ Tech Stack

![Kotlin](https://img.shields.io/badge/kotlin-%237F52FF.svg?style=for-the-badge&logo=kotlin&logoColor=white)
![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![TypeScript](https://img.shields.io/badge/typescript-%23007ACC.svg?style=for-the-badge&logo=typescript&logoColor=white)
![Vue.js](https://img.shields.io/badge/vuejs-%2335495e.svg?style=for-the-badge&logo=vuedotjs&logoColor=%234FC08D)
![Astro](https://img.shields.io/badge/astro-%23ff5d01.svg?style=for-the-badge&logo=astro&logoColor=white)
![TailwindCSS](https://img.shields.io/badge/tailwindcss-%2338B2AC.svg?style=for-the-badge&logo=tailwind-css&logoColor=white)
![Android](https://img.shields.io/badge/Android-3DDC84?style=for-the-badge&logo=android&logoColor=white)
![iOS](https://img.shields.io/badge/iOS-000000?style=for-the-badge&logo=ios&logoColor=white)
![Node.js](https://img.shields.io/badge/node.js-6DA55F?style=for-the-badge&logo=node.js&logoColor=white)
![pnpm](https://img.shields.io/badge/pnpm-%234a4a4a.svg?style=for-the-badge&logo=pnpm&logoColor=f69220)
![Docker](https://img.shields.io/badge/docker-%230db7ed.svg?style=for-the-badge&logo=docker&logoColor=white)
![Gradle](https://img.shields.io/badge/Gradle-02303A.svg?style=for-the-badge&logo=Gradle&logoColor=white)
![SQLite](https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white)

</div>

---

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
- **Standardized Identity (AIEOS)**: Support for AIEOS v1.1, allowing for portable and model-agnostic AI personas.
- **Hybrid Memory Model**: Pluggable memory backends including SQLite and MCP-backed Cerebro for high-context retrieval.
- **Rich Integrations**: First-class support for WhatsApp (via Meta Cloud API), git, npm, cargo, and more.

---

## 🛠️ Tech Stack

- **Core Logic**: [Kotlin Multiplatform (KMP)](https://kotlinlang.org/docs/multiplatform.html)
- **Agent Runtime**: [Rust](https://rust-lang.org/) (High-performance sidecars and CLI)
- **Desktop UI**: [Compose Multiplatform](https://kotlinlang.org/compose-multiplatform/)
- **Web Stack**: [Astro](https://astro.build/), [Vue 3](https://vuejs.org/), and [Tailwind CSS](https://tailwindcss.com/docs/installation/using-vite)
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
│   ├── agent-core-kmp/   # Core Kotlin Multiplatform logic & contracts
│   └── cerebro/          # MCP-backed long-term memory service
├── dev/                  # Local development environment (Docker/Sandbox)
├── gradle/               # Build logic and configurations
└── Makefile              # Standard entry point for development tasks
```

---

## 🚀 Getting Started

### Prerequisites

- **JDK 21+** (for Kotlin/KMP)
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
make dev-up      # Start proxy + Agent + Sandbox at http://corvus.localhost
./dev/cli.sh up-dashboard  # Swap the landing page for the dashboard UI on the same origin
make dev-shell   # Enter the Sandbox (Ubuntu)
make dev-down    # Stop the environment
```

---

## 📚 Documentation

Detailed documentation is available in English and Spanish:

- **English**: [Documentation Index](docs/index.mdx) | [Guides](docs/guides/)
- **Español**: [Índice de Documentación](docs/es/index.mdx) | [Guías](docs/es/guides/)

You can also build and view the full documentation site locally:

## 🔎 DeepWiki

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/dallay/corvus)

## 🤝 Contributing

Contributions are welcome! Please read our [CONTRIBUTING.md](CONTRIBUTING.md) and ensure you run `make check` before submitting a Pull Request.

---

## 📄 License

This project is licensed under the Apache License 2.0. See the [LICENSE](LICENSE) file for details.

---

_(Note: Corvus is currently in active development. Features and architecture are subject to evolution.)_
