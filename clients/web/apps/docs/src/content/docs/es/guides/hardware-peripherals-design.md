---
title: Diseño de Periféricos de Hardware
---

Corvus permite que microcontroladores (MCU) y computadoras de placa única (SBC) interpreten
comandos en lenguaje natural de forma dinámica, generen código específico de hardware y ejecuten
interacciones con periféricos en tiempo real.

## 1. Visión

**Objetivo:** Corvus funciona como un agente de IA consciente del hardware que:

- Recibe instrucciones en lenguaje natural (`"Mueve el brazo X"`, `"Enciende el LED"`) por canales
  como WhatsApp o Telegram.
- Obtiene documentación precisa del hardware (datasheets, mapas de registros).
- Sintetiza lógica/código en Rust con un LLM (Gemini o modelos open-source locales).
- Ejecuta esa lógica para manipular periféricos (GPIO, I2C, SPI).
- Persiste código optimizado para reutilizarlo en futuras solicitudes.

**Modelo mental:** Corvus es el cerebro que entiende hardware; los periféricos son los actuadores.

## 2. Dos Modos de Operación

### Modo 1: Edge-Native (Autónomo)

**Objetivo:** placas con Wi-Fi (ESP32, Raspberry Pi).

Corvus corre directamente en el dispositivo. La placa inicia un servidor gRPC/nanoRPC y se
comunica localmente con los periféricos.

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ Corvus en ESP32 / Raspberry Pi (Edge-Native)                             │
│                                                                             │
│ ┌─────────────┐    ┌──────────────┐    ┌─────────────────────────────────┐ │
│ │ Canales     │───►│ Bucle Agente │───►│ RAG: datasheets, registros      │ │
│ │ WhatsApp    │    │ (LLM calls)  │    │ → contexto del LLM              │ │
│ │ Telegram    │    └──────┬───────┘    └─────────────────────────────────┘ │
│ └─────────────┘           │                                                 │
│                           ▼                                                 │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Síntesis código → Wasm/exec dinámica → GPIO/I2C/SPI → persistencia    │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ servidor gRPC/nanoRPC ◄──► Periféricos (GPIO, I2C, SPI, sensores, actuadores) │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Flujo:**

1. Usuario envía por WhatsApp: `"Enciende el LED del pin 13"`.
2. Corvus obtiene documentación específica de la placa.
3. El LLM genera código Rust.
4. El código se ejecuta en sandbox (Wasm o enlace dinámico).
5. Se conmuta GPIO y se devuelve el resultado.
6. El código optimizado se persiste para futuras solicitudes.

Todo ocurre en el dispositivo. No requiere host.

### Modo 2: Host-Mediated (Desarrollo / Depuración)

**Objetivo:** hardware conectado por USB / J-Link / Aardvark a un host (macOS, Linux).

Corvus se ejecuta en el host y mantiene un enlace consciente del hardware con el objetivo. Se
usa para desarrollo, introspección y flashing.

```text
┌─────────────────────┐                    ┌──────────────────────────────────┐
│ Corvus en Mac     │ USB / J-Link /     │ STM32 Nucleo-F401RE              │
│                     │ Aardvark           │ (u otro MCU)                      │
│ - Canales           │ ◄────────────────► │ - Mapa de memoria                 │
│ - LLM               │                    │ - Periféricos (GPIO, ADC, I2C)    │
│ - Sonda HW          │ descubrimiento     │ - Flash / RAM                     │
│ - Flash / debug     │ VID/PID            │                                    │
└─────────────────────┘                    └──────────────────────────────────┘
```

**Ejemplos de flujo:**

1. Usuario envía por Telegram: `"¿Qué direcciones de memoria se pueden leer en este USB?"`
2. Corvus identifica hardware (VID/PID y arquitectura).
3. Hace mapeo de memoria y sugiere espacios disponibles.
4. Devuelve el resultado al usuario.

O:

1. Usuario: `"Flashea este firmware al Nucleo"`.
2. Corvus escribe/flashea con OpenOCD o `probe-rs`.
3. Confirma éxito.

O:

1. Auto-descubrimiento: `"STM32 Nucleo en /dev/ttyACM0, ARM Cortex-M4"`.
2. Sugiere capacidades (`GPIO`, `ADC`, `flash`) y pide siguiente acción.

### Comparativa de Modos

| Aspecto            | Edge-Native              | Host-Mediated             |
|--------------------|--------------------------|---------------------------|
| Dónde corre Corvus | Dispositivo (ESP32, RPi) | Host (macOS, Linux)       |
| Enlace hardware    | Local (GPIO, I2C, SPI)   | USB, J-Link, Aardvark     |
| LLM                | En dispositivo o nube    | En host (nube o local)    |
| Caso de uso        | Producción, autónomo     | Dev, debug, introspección |
| Canales            | WhatsApp, etc. (Wi-Fi)   | Telegram, CLI, etc.       |

## 3. Modos Legados / Simples (Antes de LLM en Edge)

Para placas sin Wi-Fi o antes de terminar Edge-Native:

### Modo A: Host + Periférico remoto (STM32 por serial)

Host ejecuta Corvus y el periférico un firmware mínimo. JSON sobre serial.

### Modo B: Raspberry Pi como host (GPIO nativo)

Corvus en Raspberry Pi con GPIO vía `rppal` o `sysfs`. Sin firmware separado.

## 4. Requisitos Técnicos

| Requisito               | Descripción                                                                          |
|-------------------------|--------------------------------------------------------------------------------------|
| Lenguaje                | Rust puro. Usar `no_std` cuando aplique en embebidos.                                |
| Comunicación            | gRPC o nanoRPC ligero para baja latencia.                                            |
| Ejecución dinámica      | Ejecutar lógica generada por LLM de forma segura (Wasm o enlace dinámico soportado). |
| Recuperación documental | Pipeline RAG con datasheets, registros y pinouts en contexto del LLM.                |
| Descubrimiento hardware | Identificación USB por VID/PID y detección de arquitectura (ARM, RISC-V, etc.).      |

### Pipeline RAG (Recuperación de Datasheets)

- **Indexar:** datasheets, manuales y registros (PDF a chunks + embeddings).
- **Recuperar:** ante `"enciende LED"`, buscar snippets relevantes de la placa.
- **Inyectar:** añadir snippets al prompt/contexto del LLM.
- **Resultado:** código específico y correcto para la placa.

### Opciones de Ejecución Dinámica

| Opción                  | Ventajas                           | Contras                                        |
|-------------------------|------------------------------------|------------------------------------------------|
| Wasm                    | Aislado, portable, sin FFI directo | Sobrecarga; acceso HW limitado                 |
| Enlace dinámico         | Velocidad nativa, acceso total     | Específico de plataforma; riesgos de seguridad |
| DSL interpretado        | Seguro, auditable                  | Más lento; menos expresivo                     |
| Templates precompilados | Rápido, seguro                     | Menos flexible; mantenimiento de plantillas    |

**Recomendación:** comenzar con templates precompilados y parametrización. Evolucionar a Wasm
cuando la lógica definida por usuario esté estable.

## 5. CLI y Configuración

### Flags CLI

```bash
# Edge-Native: ejecutar en dispositivo (ESP32, RPi)
corvus agent --mode edge

# Host-Mediated: conectar objetivo USB/J-Link
corvus agent --peripheral nucleo-f401re:/dev/ttyACM0
corvus agent --probe jlink

# Introspección de hardware
corvus hardware discover
corvus hardware introspect /dev/ttyACM0
```

### Config (`config.toml`)

```toml
[peripherals]
enabled = true
mode = "host"  # "edge" | "host"
datasheet_dir = "docs/datasheets"  # documentos RAG para contexto LLM

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

## 6. Arquitectura: Periférico como Punto de Extensión

### Nuevo Trait: `Peripheral`

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

### Flujo

1. **Inicio:** Corvus carga config y lee `peripherals.boards`.
2. **Conexión:** crea una implementación `Peripheral` por placa y llama `connect()`.
3. **Herramientas:** combina herramientas de periféricos conectados con las default.
4. **Bucle agente:** llamadas como `gpio_write` o `sensor_read` delegan al periférico.
5. **Apagado:** llama `disconnect()` en cada periférico.

### Soporte de Placas

| Placa           | Transporte | Firmware / Driver | Herramientas                          |
|-----------------|------------|-------------------|---------------------------------------|
| `nucleo-f401re` | serial     | Zephyr / Embassy  | `gpio_read`, `gpio_write`, `adc_read` |
| `rpi-gpio`      | nativo     | `rppal` o `sysfs` | `gpio_read`, `gpio_write`             |
| `esp32`         | serial/ws  | ESP-IDF / Embassy | GPIO, Wi-Fi, MQTT                     |

## 7. Protocolos de Comunicación

### gRPC / nanoRPC

Para RPC tipado y de baja latencia entre Corvus y periféricos:

- `nanoRPC` o `tonic` (gRPC) con servicios definidos en Protobuf.
- Métodos: `GpioWrite`, `GpioRead`, `I2cTransfer`, `SpiTransfer`, `MemoryRead`, `FlashWrite`, etc.
- Soporta streaming, llamadas bidireccionales y generación de código desde `.proto`.

### Fallback serial (Host-Mediated, legado)

JSON simple sobre serial para placas sin soporte gRPC:

**Request (host a periférico):**

```json
{"id":"1","cmd":"gpio_write","args":{"pin":13,"value":1}}
```

**Response (periférico a host):**

```json
{"id":"1","ok":true,"result":"done"}
```

## 8. Firmware (Repo o Crate Separado)

- `corvus-firmware` o `corvus-peripheral` como crate/workspace separado.
- Targets: `thumbv7em-none-eabihf` (STM32), `armv7-unknown-linux-gnueabihf` (RPi), etc.
- Usa `embassy` o Zephyr para STM32.
- Implementa el protocolo anterior.
- El usuario lo flashea en la placa; luego Corvus conecta y descubre capacidades.

## 9. Fases de Implementación

### Fase 1: Skeleton (completada)

- [x] Añadir trait `Peripheral`, esquema config y CLI (`corvus peripheral list/add`)
- [x] Añadir flag `--peripheral` al agente
- [x] Documentar en `AGENTS.md`

### Fase 2: Descubrimiento Host-Mediated (completada)

- [x] `corvus hardware discover`: enumeración USB (VID/PID)
- [x] Registro de placas: mapeo VID/PID a arquitectura y nombre
- [x] `corvus hardware introspect <path>`: mapa de memoria y periféricos

### Fase 3: Host-Mediated Serial / J-Link

- [x] `SerialPeripheral` para STM32 sobre USB CDC
- [ ] Integración `probe-rs` u OpenOCD para flash/debug
- [x] Herramientas: `gpio_read`, `gpio_write` (`memory_read`, `flash_write` en futuro)

### Fase 4: Pipeline RAG (completada)

- [x] Índice de datasheets (markdown/texto a chunks)
- [x] Recuperar e inyectar en consultas de hardware
- [x] Aumento de prompt específico por placa

**Uso:** agregar `datasheet_dir = "docs/datasheets"` a `[peripherals]` en `config.toml`. Colocar
archivos `.md` o `.txt` por placa (`nucleo-f401re.md`, `rpi-gpio.md`). Archivos en `_generic/` o
`generic.md` aplican a todas.

### Fase 5: Edge-Native RPi (completada)

- [x] Corvus en Raspberry Pi (GPIO nativo con `rppal`)
- [ ] Servidor gRPC/nanoRPC para acceso local a periféricos
- [ ] Persistencia de código (snippets sintetizados)

### Fase 6: Edge-Native ESP32

- [x] ESP32 host-mediated (serial), mismo protocolo JSON que STM32
- [x] Crate firmware `corvus-esp32` (`firmware/corvus-esp32`) para GPIO por UART
- [x] ESP32 en registro de hardware (CH340 VID/PID)
- [ ] Corvus en ESP32 (Wi-Fi + LLM, edge-native)
- [ ] Ejecución Wasm o basada en templates para lógica generada por LLM

### Fase 7: Ejecución Dinámica (Código generado por LLM)

- [ ] Librería de templates parametrizados GPIO/I2C/SPI
- [ ] Runtime Wasm opcional para lógica definida por usuario (sandbox)
- [ ] Persistir y reutilizar rutas de código optimizadas

## 10. Consideraciones de Seguridad

- Validar `path` serial con allowlist (`/dev/ttyACM*`, `/dev/ttyUSB*`), nunca paths arbitrarios.
- Restringir pines GPIO expuestos; evitar pines de alimentación/reset.
- No almacenar secretos en firmware periférico; el host gestiona autenticación.

## 11. No Objetivos (Por Ahora)

- Ejecutar Corvus completo sobre STM32 bare-metal (sin Wi-Fi y poca RAM): usar Host-Mediated.
- Garantías hard real-time: periféricos se consideran best-effort.
- Ejecución de código nativo arbitrario desde LLM: priorizar Wasm o templates.

## 12. Documentos Relacionados

- [Estructura](./structure.md)
- [Flujo de Desarrollo](./development.md)
- [Estructura del Proyecto](./structure.md)

## 13. Referencias

- [Soporte Rust en Zephyr RTOS](https://docs.zephyrproject.org/latest/develop/languages/rust/index.html)
- [Embassy](https://embassy.dev/)
- [rppal](https://github.com/golemparts/rppal)
- [STM32 Nucleo-F401RE](https://docs.zephyrproject.org/latest/boards/st/nucleo_f401re/doc/index.html)
- [tonic](https://github.com/hyperium/tonic)
- [probe-rs](https://probe.rs/)
- [nusb](https://github.com/kevinmehall/nusb)
