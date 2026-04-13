---
title: Getting Started
description: Set up Corvus locally with the supported toolchain and canonical first-run commands.
owner: team-platform
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: guide
---

Welcome to **Corvus**. This guide helps you run the customized project baseline locally.

## Prerequisites

- **Java JDK 21** or higher.
- **Rust 1.75** or higher.
- **Node.js 22** or higher.
- **pnpm 10** or higher.
- **Git**.
- A bash-compatible shell (Linux, macOS, or Git Bash on Windows).
- **Docker**: required only for sandbox and development containers; if you don't use containers,
  Docker is not needed.

The listed tools are required for running `make setup` and `make build` when using those
containerized workflows.

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

## Interactive Onboarding and Dashboard Activation

Run the interactive setup when you want guided first-run configuration:

```bash
corvus onboard --interactive
```

At the end of the wizard (after summary and any channel launch prompt), Corvus asks:

- `Activate web dashboard now? (optional)`

If you accept, Corvus prints a one-screen activation guide with canonical local defaults:

- Local entrypoint: `http://corvus.localhost`
- Gateway check URL: `http://corvus.localhost/api/health`
- API gateway base path: `/api`
- Pairing path: proxied via `/api/pair`

If you decline, Corvus keeps the CLI-only path and prints a resume-later block with exact commands.

## Troubleshooting and Resume Later

When activation cannot complete immediately, Corvus prints deterministic diagnosis codes:

- `DASH-001 GatewayNotRunning`
- `DASH-002 GatewayRunningPairingRequired`
- `DASH-003 GatewayRunningAlreadyPaired`
- `DASH-004 DashboardUiUnavailable`
- `DASH-999 UnknownLocalFailure`

Use this safe, copy-paste resume flow anytime:

```bash
corvus status
corvus gateway
# from Corvus repository root (source checkout):
make dev-up
./dev/cli.sh up-dashboard
# then open http://corvus.localhost and complete pairing through /api/pair
```

If you need command help:

```bash
corvus --help
```

## Next Steps

- Review [Project Structure](./structure).
- Check [Features Checklist](./features).
- Continue with [Development](./development).
