---
title: Hardware Tools
description: Reference for hardware discovery, memory mapping, and peripheral control in Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: reference
---

Hardware tools enable agents to interact with connected microcontrollers (MCUs) and single-board computers (SBCs) for introspection, debugging, and physical control.

## Discovery & Information

### `hardware_board_info`
Returns high-level details about connected boards, including chip name, architecture, and static datasheet descriptions.
- **Security Tier:** Read-Only (Safe).
- **Supported Boards:** STM32 Nucleo, Arduino Uno, ESP32, Raspberry Pi.

### `hardware_capabilities`
Queries connected hardware for reported capabilities like available GPIO pins and LED identifiers.
- **Security Tier:** Read-Only (Safe).

---

## Memory Operations

### `hardware_memory_map`
Returns flash and RAM address ranges for connected boards. Uses `probe-rs` for live introspection when available or falls back to static datasheet maps.
- **Security Tier:** Read-Only (Safe).

### `hardware_memory_read`
Reads actual memory or register values from supported Nucleo boards via USB. Supported boards: `nucleo-f401re`, `nucleo-f411re`.
- **Security Tier:** Read-Only (Safe).
- **Requirements:** Requires the `probe` feature and a supported debugger connection.
- **Parameters:** `address` (hex), `length` (bytes), `board`.

---

## Peripheral Control

### `gpio_read`
Reads the current digital value (0 or 1) of a specific GPIO pin.
- **Security Tier:** Read-Only (Safe).
- **Platforms:** Raspberry Pi (native), Arduino Uno Q (via bridge).

### `gpio_write`
Sets a GPIO pin to high (1) or low (0).
- **Security Tier:** Action-Bearing (Risk-bearing).
- **Platforms:** Raspberry Pi (native), Arduino Uno Q (via bridge).

### `arduino_upload`
Compiles and uploads an agent-generated Arduino sketch (.ino) to a connected board.
- **Security Tier:** Action-Bearing (Risk-bearing).
- **Requirements:** Requires `arduino-cli` installed on the host system.
- **Usage:** Used for dynamic tasks like "blink the LED" or "display a heart on the grid".
