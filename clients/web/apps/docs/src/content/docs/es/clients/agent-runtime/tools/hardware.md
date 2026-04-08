---
title: Herramientas de Hardware
description: Referencia para el descubrimiento de hardware, mapeo de memoria y control de periféricos en Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: reference
---

Las herramientas de hardware permiten a los agentes interactuar con microcontroladores (MCUs) y computadoras de placa única (SBCs) conectadas para introspección, depuración y control físico.

## Descubrimiento e Información

### `hardware_board_info`
Devuelve detalles de alto nivel sobre las placas conectadas, incluyendo el nombre del chip, la arquitectura y descripciones estáticas de las hojas de datos.
- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Placas Soportadas:** STM32 Nucleo, Arduino Uno, ESP32, Raspberry Pi.

### `hardware_capabilities`
Consulta al hardware conectado sus capacidades reportadas, como los pines GPIO disponibles e identificadores de LED.
- **Nivel de Seguridad:** Solo Lectura (Segura).

---

## Operaciones de Memoria

### `hardware_memory_map`
Devuelve los rangos de direcciones de Flash y RAM para las placas conectadas. Utiliza `probe-rs` para introspección en vivo cuando está disponible o recurre a mapas estáticos de hojas de datos.
- **Nivel de Seguridad:** Solo Lectura (Segura).

### `hardware_memory_read`
Lee valores reales de memoria o registros de un objetivo conectado (ej. STM32) a través de USB.
- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Requisitos:** Requiere la funcionalidad `probe` y una conexión de depurador soportada.
- **Parámetros:** `address` (hex), `length` (bytes), `board`.

---

## Control de Periféricos

### `gpio_read`
Lee el valor digital actual (0 o 1) de un pin GPIO específico.
- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Plataformas:** Raspberry Pi (nativo), Arduino Uno Q (vía bridge).

### `gpio_write`
Establece un pin GPIO en alto (1) o bajo (0).
- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Plataformas:** Raspberry Pi (nativo), Arduino Uno Q (vía bridge).

### `arduino_upload`
Compila y carga un sketch de Arduino (.ino) generado por el agente en una placa conectada.
- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Requisitos:** Requiere `arduino-cli` instalado en el sistema host.
- **Uso:** Se utiliza para tareas dinámicas como "parpadea el LED" o "muestra un corazón en la matriz".
