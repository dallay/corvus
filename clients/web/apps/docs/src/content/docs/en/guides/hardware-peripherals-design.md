---
title: Hardware Peripherals Design
---

Corvus enables microcontrollers (MCUs) and single-board computers (SBCs) to dynamically
interpret natural-language commands, generate hardware-specific code, and execute peripheral
interactions in real time.

## 1. Vision

**Goal:** Corvus acts as a hardware-aware AI agent that:

- Receives natural-language triggers (`"Move X arm"`, `"Turn on LED"`) via channels
  (WhatsApp, Telegram).
- Fetches accurate hardware documentation (datasheets, register maps).
- Synthesizes Rust code/logic with an LLM (Gemini, local open-source models).
- Executes the logic to manipulate peripherals (GPIO, I2C, SPI).
- Persists optimized code for future reuse.

**Mental model:** Corvus is the hardware-aware brain; peripherals are the actuators it controls.

## 2. Two Operation Modes

### Mode 1: Edge-Native (Standalone)

**Target:** Wi-Fi-enabled boards (ESP32, Raspberry Pi).

Corvus runs directly on the device. The board starts a gRPC/nanoRPC server and communicates
with peripherals locally.

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ Corvus on ESP32 / Raspberry Pi (Edge-Native)                             │
│                                                                             │
│ ┌─────────────┐    ┌──────────────┐    ┌─────────────────────────────────┐ │
│ │ Channels    │───►│ Agent Loop   │───►│ RAG: datasheets, register maps  │ │
│ │ WhatsApp    │    │ (LLM calls)  │    │ → LLM context                    │ │
│ │ Telegram    │    └──────┬───────┘    └─────────────────────────────────┘ │
│ └─────────────┘           │                                                 │
│                           ▼                                                 │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Code synthesis → Wasm / dynamic exec → GPIO / I2C / SPI → persist     │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ gRPC/nanoRPC server ◄──► Peripherals (GPIO, I2C, SPI, sensors, actuators)  │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Workflow:**

1. User sends WhatsApp: `"Turn on LED on pin 13"`.
2. Corvus fetches board-specific docs (for example, ESP32 GPIO mapping).
3. The LLM synthesizes Rust code.
4. Code runs in a sandbox (Wasm or dynamic linking).
5. GPIO is toggled and the result is returned.
6. Optimized code is persisted for future `"Turn on LED"` requests.

All steps happen on-device. No host required.

### Mode 2: Host-Mediated (Development / Debugging)

**Target:** Hardware connected by USB / J-Link / Aardvark to a host (macOS, Linux).

Corvus runs on the host and keeps a hardware-aware link to the target. This is used for
development, introspection, and flashing.

```text
┌─────────────────────┐                    ┌──────────────────────────────────┐
│ Corvus on Mac     │ USB / J-Link /     │ STM32 Nucleo-F401RE              │
│                     │ Aardvark           │ (or other MCU)                    │
│ - Channels          │ ◄────────────────► │ - Memory map                      │
│ - LLM               │                    │ - Peripherals (GPIO, ADC, I2C)    │
│ - Hardware probe    │ VID/PID discovery  │ - Flash / RAM                     │
│ - Flash / debug     │                    │                                    │
└─────────────────────┘                    └──────────────────────────────────┘
```

**Workflow examples:**

1. User sends Telegram: `"What are the readable memory addresses on this USB device?"`
2. Corvus identifies hardware (VID/PID, architecture).
3. It performs memory mapping and suggests address spaces.
4. It returns results to the user.

Or:

1. User asks: `"Flash this firmware to the Nucleo"`.
2. Corvus flashes with OpenOCD or `probe-rs`.
3. It confirms success.

Or:

1. Corvus auto-discovers: `"STM32 Nucleo on /dev/ttyACM0, ARM Cortex-M4"`.
2. It suggests capabilities (`GPIO`, `ADC`, `flash`) and asks what to do next.

### Mode Comparison

| Aspect         | Edge-Native            | Host-Mediated             |
|----------------|------------------------|---------------------------|
| Corvus runs on | Device (ESP32, RPi)    | Host (macOS, Linux)       |
| Hardware link  | Local (GPIO, I2C, SPI) | USB, J-Link, Aardvark     |
| LLM            | On-device or cloud     | Host (cloud or local)     |
| Use case       | Production, standalone | Dev, debug, introspection |
| Channels       | WhatsApp, etc. (Wi-Fi) | Telegram, CLI, etc.       |

## 3. Legacy / Simpler Modes (Pre LLM-on-Edge)

For boards without Wi-Fi, or before full Edge-Native is ready:

### Mode A: Host + Remote Peripheral (STM32 over serial)

Host runs Corvus; peripheral runs minimal firmware. JSON over serial.

### Mode B: Raspberry Pi as Host (Native GPIO)

Corvus runs on Raspberry Pi; GPIO access via `rppal` or `sysfs`. No separate firmware.

## 4. Technical Requirements

| Requirement             | Description                                                                         |
|-------------------------|-------------------------------------------------------------------------------------|
| Language                | Pure Rust. Use `no_std` where applicable on embedded targets.                       |
| Communication           | Lightweight gRPC or nanoRPC for low-latency command execution.                      |
| Dynamic execution       | Run LLM-generated logic safely (Wasm isolation or dynamic linking where supported). |
| Documentation retrieval | RAG pipeline for datasheets, register maps, and pinouts in LLM context.             |
| Hardware discovery      | VID/PID identification for USB devices; architecture detection (ARM, RISC-V, etc.). |

### RAG Pipeline (Datasheet Retrieval)

- **Index:** Datasheets, reference manuals, register maps (PDF to chunks + embeddings).
- **Retrieve:** For a query like `"turn on LED"`, fetch relevant board snippets.
- **Inject:** Add snippets to LLM system prompt/context.
- **Result:** The LLM generates accurate board-specific code.

### Dynamic Execution Options

| Option                | Pros                               | Cons                                  |
|-----------------------|------------------------------------|---------------------------------------|
| Wasm                  | Sandboxed, portable, no direct FFI | Overhead; constrained hardware access |
| Dynamic linking       | Native speed, full access          | Platform-specific; security risks     |
| Interpreted DSL       | Safe, auditable                    | Slower; less expressive               |
| Precompiled templates | Fast, secure                       | Less flexible; template maintenance   |

**Recommendation:** Start with precompiled templates + parameterization. Move to Wasm for
user-defined logic when stable.

## 5. CLI and Config

### CLI Flags

```bash
# Edge-Native: run on device (ESP32, RPi)
corvus agent --mode edge

# Host-Mediated: connect to USB/J-Link target
corvus agent --peripheral nucleo-f401re:/dev/ttyACM0
corvus agent --probe jlink

# Hardware introspection
corvus hardware discover
corvus hardware introspect /dev/ttyACM0
```

### Config (`config.toml`)

```toml
[peripherals]
enabled = true
mode = "host"  # "edge" | "host"
datasheet_dir = "docs/datasheets"  # RAG docs for LLM context

[[peripherals.boards]]
board = "nucleo-f401re"
transport = "serial"
path = "/dev/ttyACM0"
baud = 115200

[[peripherals.boards]]
board = "rpi-gpio"
transport = "native"

[[peripherals.boards]]
board = "esp32"
transport = "wifi"
```

## 6. Architecture: Peripheral as Extension Point

### New Trait: `Peripheral`

```rust
/// A hardware peripheral that exposes capabilities as tools.
#[async_trait]
pub trait Peripheral: Send + Sync {
    fn name(&self) -> &str;
    fn board_type(&self) -> &str; // e.g. "nucleo-f401re", "rpi-gpio"
    async fn connect(&mut self) -> anyhow::Result<()>;
    async fn disconnect(&mut self) -> anyhow::Result<()>;
    async fn health_check(&self) -> bool;
    /// Tools this peripheral provides (gpio_read, gpio_write, sensor_read, etc.)
    fn tools(&self) -> Vec<Box<dyn Tool>>;
}
```

### Flow

1. **Startup:** Corvus loads config and reads `peripherals.boards`.
2. **Connect:** For each board, create a `Peripheral` implementation and call `connect()`.
3. **Tools:** Collect tools from connected peripherals and merge with default tools.
4. **Agent loop:** Calls like `gpio_write` and `sensor_read` delegate to the peripheral.
5. **Shutdown:** Call `disconnect()` on each peripheral.

### Board Support

| Board           | Transport | Firmware / Driver  | Tools                                 |
|-----------------|-----------|--------------------|---------------------------------------|
| `nucleo-f401re` | serial    | Zephyr / Embassy   | `gpio_read`, `gpio_write`, `adc_read` |
| `rpi-gpio`      | native    | `rppal` or `sysfs` | `gpio_read`, `gpio_write`             |
| `esp32`         | serial/ws | ESP-IDF / Embassy  | GPIO, Wi-Fi, MQTT                     |

## 7. Communication Protocols

### gRPC / nanoRPC

For low-latency, typed RPC between Corvus and peripherals:

- `nanoRPC` or `tonic` (gRPC) with Protobuf-defined services.
- Methods: `GpioWrite`, `GpioRead`, `I2cTransfer`, `SpiTransfer`, `MemoryRead`, `FlashWrite`, etc.
- Supports streaming, bidirectional calls, and code generation from `.proto` files.

### Serial Fallback (Host-Mediated, legacy)

Simple JSON over serial for boards without gRPC support:

**Request (host to peripheral):**

```json
{"id":"1","cmd":"gpio_write","args":{"pin":13,"value":1}}
```

**Response (peripheral to host):**

```json
{"id":"1","ok":true,"result":"done"}
```

## 8. Firmware (Separate Repo or Crate)

- `corvus-firmware` or `corvus-peripheral` as a separate crate/workspace.
- Targets: `thumbv7em-none-eabihf` (STM32), `armv7-unknown-linux-gnueabihf` (RPi), etc.
- Uses `embassy` or Zephyr for STM32.
- Implements the protocol above.
- User flashes it to the board; Corvus then connects and discovers capabilities.

## 9. Implementation Phases

### Phase 1: Skeleton (Done)

- [x] Add `Peripheral` trait, config schema, CLI (`corvus peripheral list/add`)
- [x] Add `--peripheral` flag to agent
- [x] Document in `AGENTS.md`

### Phase 2: Host-Mediated Hardware Discovery (Done)

- [x] `corvus hardware discover`: enumerate USB devices (VID/PID)
- [x] Board registry: map VID/PID to architecture and board name
- [x] `corvus hardware introspect <path>`: memory map, peripheral list

### Phase 3: Host-Mediated Serial / J-Link

- [x] `SerialPeripheral` for STM32 over USB CDC
- [ ] `probe-rs` or OpenOCD integration for flash/debug
- [x] Tools: `gpio_read`, `gpio_write` (`memory_read`, `flash_write` in future)

### Phase 4: RAG Pipeline (Done)

- [x] Datasheet index (markdown/text to chunks)
- [x] Retrieve-and-inject on hardware-related queries
- [x] Board-specific prompt augmentation

**Usage:** Add `datasheet_dir = "docs/datasheets"` to `[peripherals]` in `config.toml`. Place
`.md` or `.txt` files named by board (`nucleo-f401re.md`, `rpi-gpio.md`). Files in `_generic/`
or named `generic.md` apply to all boards.

### Phase 5: Edge-Native RPi (Done)

- [x] Corvus on Raspberry Pi (native GPIO via `rppal`)
- [ ] gRPC/nanoRPC server for local peripheral access
- [ ] Code persistence (store synthesized snippets)

### Phase 6: Edge-Native ESP32

- [x] Host-mediated ESP32 (serial transport) with the same JSON protocol as STM32
- [x] `corvus-esp32` firmware crate (`firmware/corvus-esp32`) for GPIO over UART
- [x] ESP32 in hardware registry (CH340 VID/PID)
- [ ] Corvus on ESP32 (Wi-Fi + LLM, edge-native)
- [ ] Wasm or template-based execution for LLM-generated logic

### Phase 7: Dynamic Execution (LLM-Generated Code)

- [ ] Template library: parameterized GPIO/I2C/SPI snippets
- [ ] Optional Wasm runtime for user-defined logic (sandboxed)
- [ ] Persist and reuse optimized code paths

## 10. Security Considerations

- Validate serial `path` using an allowlist (`/dev/ttyACM*`, `/dev/ttyUSB*`), never arbitrary
  paths.
- Restrict exposed GPIO pins; avoid power/reset pins.
- Keep secrets out of peripheral firmware; host handles authentication.

## 11. Non-Goals (For Now)

- Running full Corvus on bare STM32 (no Wi-Fi, limited RAM): use Host-Mediated mode.
- Real-time guarantees: peripherals are best-effort.
- Arbitrary native code execution from LLM: prefer Wasm or templates.

## 12. Related Documents

- [Architecture](./architecture.md)
- [Development Workflow](./development.md)
- [Project Structure](./structure.md)

## 13. References

- [Zephyr RTOS Rust support](https://docs.zephyrproject.org/latest/develop/languages/rust/index.html)
- [Embassy](https://embassy.dev/)
- [rppal](https://github.com/golemparts/rppal)
- [STM32 Nucleo-F401RE](https://docs.zephyrproject.org/latest/boards/st/nucleo_f401re/doc/index.html)
- [tonic](https://github.com/hyperium/tonic)
- [probe-rs](https://probe.rs/)
- [nusb](https://github.com/kevinmehall/nusb)
