---
title: Referencia de la CLI
description: Guía completa de los comandos y opciones de la CLI del agente Corvus.
---

La CLI de Corvus (`corvus`) es la interfaz principal para gestionar tus agentes, hardware y servicios.

## Comandos Principales

### `onboard`
Inicializa tu espacio de trabajo y configuración.

- `--interactive`: Ejecuta el asistente interactivo completo (por defecto es configuración rápida).
- `--channels-only`: Reconfigura solo los canales.
- `--api-key <KEY>`: Clave de API para configuración rápida.
- `--provider <NAME>`: Nombre del proveedor (por defecto: openrouter).
- `--memory <TYPE>`: Backend de memoria (sqlite, lucid, surreal-graphs, surreal, markdown, none).

**Ejemplo:**
```bash
corvus onboard --interactive
```

### `agent`
Inicia el bucle del agente de IA.

- `-m, --message <TEXT>`: Modo de mensaje único.
- `-p, --provider <NAME>`: Proveedor a utilizar.
- `--model <MODEL>`: Modelo específico a utilizar.
- `-t, --temperature <VALUE>`: Temperatura (0.0 - 2.0).
- `--peripheral <BOARD:PATH>`: Conecta un periférico (ej., `nucleo-f401re:/dev/ttyACM0`).

**Ejemplo:**
```bash
corvus agent -m "Hola, ¿cómo puedes ayudarme hoy?"
```

### `daemon`
Inicia el runtime autónomo de larga duración (gateway + canales + heartbeat + planificador).

- `-p, --port <PORT>`: Puerto para escuchar.
- `--host <HOST>`: Host para vincularse.

**Ejemplo:**
```bash
corvus daemon --port 3000
```

### `gateway`
Inicia solo el servidor gateway (webhooks, websockets).

- `-p, --port <PORT>`: Puerto para escuchar.
- `--host <HOST>`: Host para vincularse.

**Ejemplo:**
```bash
corvus gateway --port 3001
```

---

## Sistema y Servicio

### `status`
Muestra los detalles completos del estado del sistema.

**Ejemplo:**
```bash
corvus status
```

### `doctor`
Ejecuta diagnósticos para el daemon, el planificador y la frescura de los canales.

**Ejemplo:**
```bash
corvus doctor
```

### `service`
Gestiona el ciclo de vida del servicio del SO (systemd/launchd).

- `install`: Instala la unidad de servicio del daemon.
  - `--linger <MODE>`: Solo Linux: mantiene el servicio activo (Keep, On, Off).
- `start`: Inicia el servicio.
- `stop`: Detiene el servicio.
- `restart`: Reinicia el servicio.
- `status`: Verifica el estado del servicio.
- `uninstall`: Elimina la unidad de servicio.

**Ejemplo:**
```bash
corvus service install --linger on
```

---

## Gestión de Tareas

### `cron`
Configura y gestiona tareas programadas.

- `list`: Lista todas las tareas programadas.
- `add <EXPR> <CMD>`: Añade una tarea usando una expresión cron.
- `add-at <TIMESTAMP> <CMD>`: Añade una tarea de un solo uso en un tiempo RFC3339 específico.
- `add-every <MS> <CMD>`: Añade una tarea de intervalo fijo.
- `once <DELAY> <CMD>`: Añade una tarea retrasada de un solo uso (ej., "30m", "2h").
- `remove <ID>`: Elimina una tarea.
- `pause <ID>`: Pausa una tarea.
- `resume <ID>`: Reanuda una tarea pausada.

**Ejemplo:**
```bash
corvus cron add "0 9 * * *" "corvus agent -m 'Actualización diaria'"
```

---

## Proveedores y Autenticación

### `providers`
Lista todos los proveedores de IA soportados.

**Ejemplo:**
```bash
corvus providers
```

### `auth`
Gestiona los perfiles de autenticación de los proveedores.

- `login --provider <NAME>`: Inicia sesión con OAuth (ej., `openai-codex`).
- `list`: Lista los perfiles de autenticación.
- `status`: Muestra el estado de autenticación y el vencimiento de los tokens.
- `use --provider <NAME> --profile <ID>`: Establece el perfil activo.
- `logout --provider <NAME>`: Elimina un perfil.

**Ejemplo:**
```bash
corvus auth list
```

### `models`
Gestiona los catálogos de modelos de los proveedores.

- `refresh`: Actualiza y cachea los modelos del proveedor.
  - `--force`: Fuerza la actualización en vivo.

**Ejemplo:**
```bash
corvus models refresh --provider anthropic
```

---

## Capacidades e Integraciones

### `skills`
Gestiona las capacidades definidas por el usuario.

- `list`: Lista las habilidades instaladas.
- `install <SOURCE>`: Instala desde una URL de GitHub o una ruta local.
- `remove <NAME>`: Elimina una habilidad.

**Ejemplo:**
```bash
corvus skills install https://github.com/user/my-skill
```

### `plugins`
Gestiona los plugins del runtime firmados.

- `list`: Lista los plugins instalados.
- `install <ID>`: Instala un plugin (ej., `memory.surreal.graphs`).
- `remove <ID>`: Elimina un plugin.
- `verify`: Verifica la integridad del plugin.

**Ejemplo:**
```bash
corvus plugins install memory.surreal.graphs
```

### `integrations`
Explora las integraciones disponibles.

- `info <NAME>`: Muestra detalles sobre una integración específica.

**Ejemplo:**
```bash
corvus integrations info telegram
```

---

## Comunicación

### `channel`
Gestiona los canales de comunicación (Telegram, Discord, Slack).

- `list`: Lista los canales configurados.
- `start`: Inicia todos los canales configurados.
- `add <TYPE> <CONFIG_JSON>`: Añade un nuevo canal.
- `remove <NAME>`: Elimina un canal.
- `bind-telegram <IDENTITY>`: Vincula un usuario de Telegram a la lista de permitidos.

**Ejemplo:**
```bash
corvus channel list
```

---

## Hardware y Periféricos

### `hardware`
Descubre e introspecciona el hardware USB.

- `discover`: Enumera los dispositivos USB y muestra las placas conocidas.
- `introspect <PATH>`: Detalles sobre un dispositivo en una ruta específica.
- `info`: Obtiene información del chip vía USB (probe-rs).

**Ejemplo:**
```bash
corvus hardware discover
```

### `peripheral`
Gestiona periféricos de hardware (STM32, RPi, etc.).

- `list`: Lista los periféricos configurados.
- `add <BOARD> <PATH>`: Añade un periférico.
- `flash-nucleo`: Flashea el firmware de Corvus al Nucleo-F401RE.
- `flash`: Flashea el firmware de Corvus al Arduino.

**Ejemplo:**
```bash
corvus peripheral add nucleo-f401re /dev/ttyACM0
```

---

## Utilidades

### `migrate`
Migra datos desde otros runtimes de agentes.

- `openclaw`: Importa memoria desde un espacio de trabajo de OpenClaw.

**Ejemplo:**
```bash
corvus migrate openclaw --source ~/.openclaw/workspace
```
