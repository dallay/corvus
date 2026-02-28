---
title: Getting Started
---

Welcome to **Corvus**. This guide helps you run the customized project baseline locally.

## Prerequisites

- **Java JDK 21** or higher (for Kotlin/KMP).
- **Rust 1.75** or higher (for Agent Runtime).
- **Node.js 22** or higher & **pnpm 10** or higher (for Web Apps).
- **Git**.
- **Docker** (optional, for Sandboxing).
- A bash-compatible shell (Linux, macOS, or Git Bash on Windows).

## Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/dallay/corvus.git
   cd corvus
   ```

2. Run setup:

   ```bash
   make setup
   ```

## Quick Start

### Build

```bash
make build
```

### Run app

```bash
make run
```

### Run tests

```bash
make test
```

## Next Steps

- Review [Project Structure](./structure/).
- Check [Features Checklist](./features/).
- Continue with [Development](./development/).
