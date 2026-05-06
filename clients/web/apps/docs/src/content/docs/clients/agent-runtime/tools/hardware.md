---
title: Hardware Tools
description: Reference for hardware discovery, memory mapping, and peripheral control in Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-05-06
appliesTo: main
docType: reference
---

Hardware tools enable agents to interact with connected microcontrollers (MCUs) and single-board computers (SBCs) for introspection, debugging, and physical control.

## Discovery & Information

### `hardware_board_info`

Returns high-level details about connected boards, including chip name, architecture, and static datasheet descriptions.

- **Security Tier:** Read-Only (Safe).
- **Supported Boards:** STM32 Nucleo, Arduino Uno, ESP32, Raspberry Pi.

#### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `board` | `string` | Optional board name (e.g., `nucleo-f401re`). If omitted, returns info for the first configured board. |

### `hardware_capabilities`

Queries connected hardware for reported capabilities like available GPIO pins and LED identifiers.

- **Security Tier:** Read-Only (Safe).

#### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `board` | `string` | Optional board name. If omitted, queries all configured serial boards. |

---

## Memory Operations

### `hardware_memory_map`

Returns flash and RAM address ranges for connected boards. Uses `probe-rs` for live introspection when available or falls back to static datasheet maps.

- **Security Tier:** Read-Only (Safe).

#### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `board` | `string` | Optional board name (e.g., `nucleo-f401re`, `arduino-uno`). If omitted, returns map for the first configured board. |

### `hardware_memory_read`

Reads actual memory or register values from supported Nucleo boards via USB. Supported boards: `nucleo-f401re`, `nucleo-f411re`.

- **Security Tier:** Read-Only (Safe).
- **Requirements:** Requires the `probe` feature and a supported debugger connection.

#### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `address` | `string` | Memory address in hex (e.g., `0x20000000` for RAM start). Default: `0x20000000`. |
| `length` | `integer` | Number of bytes to read (default 128, max 256). |
| `board` | `string` | Board name (`nucleo-f401re`). Optional if only one is configured. |

---

## Peripheral Control

### `gpio_read`

Reads the current digital value (0 or 1) of a specific GPIO pin.

- **Security Tier:** Read-Only (Safe).
- **Platforms:** Raspberry Pi (native), Arduino Uno Q (via bridge), STM32 Nucleo (serial).

#### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `pin` | `integer` | **Required.** GPIO pin number (e.g., 13 for LED on Nucleo). |

### `gpio_write`

Sets a GPIO pin to high (1) or low (0).

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Platforms:** Raspberry Pi (native), Arduino Uno Q (via bridge), STM32 Nucleo (serial).

#### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `pin` | `integer` | **Required.** GPIO pin number. |
| `value` | `integer` | **Required.** 0 for low, 1 for high. |

### `arduino_upload`

Compiles and uploads an agent-generated Arduino sketch (.ino) to a connected board.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Requirements:** Requires `arduino-cli` installed on the host system.
- **Usage:** Used for dynamic tasks like "blink the LED" or "display a heart on the grid".

#### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `code` | `string` | **Required.** Full Arduino sketch code (complete .ino file content). |
