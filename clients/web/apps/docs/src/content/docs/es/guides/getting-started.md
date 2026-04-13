---
title: Primeros Pasos
description: Configura Corvus localmente con el toolchain soportado y los comandos canónicos de primera ejecución.
owner: team-platform
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: guide
---

Bienvenido a **Corvus**. Esta guía te ayuda a ejecutar localmente la base personalizada del
proyecto.

## Requisitos Previos

- **Java JDK 21** o superior.
- **Rust 1.75** o superior.
- **Node.js 22** o superior.
- **pnpm 10** o superior.
- **Git**.
- Un shell compatible con bash (Linux, macOS o Git Bash en Windows).
- **Docker**: requerido solo para el sandbox y los contenedores de desarrollo; si no usas
  contenedores, Docker no es necesario.

Las herramientas listadas son necesarias para ejecutar `make setup` y `make build` cuando usas esos
flujos de trabajo con contenedores.

## Instalación

1. Clona el repositorio:

   ```bash
   git clone https://github.com/dallay/corvus.git
   cd corvus
   ```

2. Ejecuta la configuración:

   ```bash
   make setup
   ```

## Inicio Rápido

### Build

```bash
make build
```

### Ejecutar app

```bash
make run
```

### Ejecutar pruebas

```bash
make test
```

## Onboarding Interactivo y Activacion del Dashboard Web

Ejecuta la configuracion interactiva cuando quieras un primer setup guiado:

```bash
corvus onboard --interactive
```

Al final del wizard (despues del resumen y cualquier prompt de canales), Corvus pregunta:

- `Activate web dashboard now? (optional)`

Si aceptas, Corvus imprime una guia de activacion en una sola pantalla con defaults locales
canonicos:

- Entrada local: `http://corvus.localhost`
- URL de chequeo del gateway: `http://corvus.localhost/api/health`
- Base URL de la API: `/api`
- Ruta de emparejamiento: enrutada por proxy mediante `/api/pair`

Si rechazas, Corvus mantiene el flujo CLI-only e imprime un bloque para retomar luego con
comandos exactos.

## Troubleshooting y Retomar Despues

Cuando la activacion no puede completarse de inmediato, Corvus imprime codigos de diagnostico
deterministas:

- `DASH-001 GatewayNotRunning`
- `DASH-002 GatewayRunningPairingRequired`
- `DASH-003 GatewayRunningAlreadyPaired`
- `DASH-004 DashboardUiUnavailable`
- `DASH-999 UnknownLocalFailure`

Usa este flujo seguro y copy-paste para retomar cuando quieras:

```bash
corvus status
corvus gateway
# desde la raiz del repositorio Corvus (source checkout):
make dev-up
./dev/cli.sh up-dashboard
# luego abre http://corvus.localhost y completa pairing por /api/pair
```

Si necesitas ayuda de comandos:

```bash
corvus --help
```

## Siguientes Pasos

> **Nota**: Las subsecciones Project Structure, Features y Development están disponibles solo en inglés. 
> Consulte la [versión en inglés](../guides/getting-started) para más detalles.
