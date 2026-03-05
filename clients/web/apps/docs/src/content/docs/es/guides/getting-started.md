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

## Siguientes Pasos

- Revisa la [Estructura del Proyecto](./structure/).
- Consulta la [Lista de Funcionalidades](./features/).
- Continúa con [Desarrollo](./development/).
