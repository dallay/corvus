---
title: Primeros Pasos
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

- URL de chequeo del gateway: `http://127.0.0.1:3000`
- URL del dashboard: `http://localhost:4324`
- Ruta de pairing: `/pair`

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
make dashboard-dev
# luego abre http://localhost:4324 y completa pairing en /pair
```

Si necesitas ayuda de comandos:

```bash
corvus --help
```

## Siguientes Pasos

- Revisa la [Estructura del Proyecto](./structure/).
- Consulta la [Lista de Funcionalidades](./features/).
- Continúa con [Desarrollo](./development/).
