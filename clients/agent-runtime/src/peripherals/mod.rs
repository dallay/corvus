//! Hardware peripherals — STM32, RPi GPIO, etc.
//!
//! Peripherals extend the agent with physical capabilities. See
//! `docs/hardware-peripherals-design.md` for the full design.

pub mod traits;

#[cfg(feature = "hardware")]
pub mod serial;

#[cfg(feature = "hardware")]
pub mod arduino_flash;
#[cfg(feature = "hardware")]
pub mod arduino_upload;
#[cfg(feature = "hardware")]
pub mod capabilities_tool;
#[cfg(feature = "hardware")]
pub mod nucleo_flash;
#[cfg(feature = "hardware")]
pub mod uno_q_bridge;
#[cfg(feature = "hardware")]
pub mod uno_q_setup;

#[cfg(all(feature = "peripheral-rpi", target_os = "linux"))]
pub mod rpi;

pub use traits::Peripheral;

use crate::config::{Config, PeripheralBoardConfig, PeripheralsConfig};
#[cfg(feature = "hardware")]
use crate::tools::HardwareMemoryMapTool;
use crate::tools::Tool;
use anyhow::Result;

/// List configured boards from config (no connection yet).
pub fn list_configured_boards(config: &PeripheralsConfig) -> Vec<&PeripheralBoardConfig> {
    if !config.enabled {
        return Vec::new();
    }
    config.boards.iter().collect()
}

/// Handle `corvus peripheral` subcommands.
#[allow(clippy::module_name_repetitions)]
pub fn handle_command(cmd: crate::PeripheralCommands, config: &Config) -> Result<()> {
    match cmd {
        crate::PeripheralCommands::List => handle_list_command(config),
        crate::PeripheralCommands::Add { board, path } => handle_add_command(board, path)?,
        #[cfg(feature = "hardware")]
        crate::PeripheralCommands::Flash { port } => {
            let port_str = arduino_flash::resolve_port(config, port.as_deref())
                .or_else(|| port.clone())
                .ok_or_else(|| anyhow::anyhow!(
                    "No port specified. Use --port /dev/cu.usbmodem* or add arduino-uno to config.toml"
                ))?;
            arduino_flash::flash_arduino_firmware(&port_str)?;
        }
        #[cfg(not(feature = "hardware"))]
        crate::PeripheralCommands::Flash { .. } => {
            println!("Arduino flash requires the 'hardware' feature.");
            println!("Build with: cargo build --features hardware");
        }
        #[cfg(feature = "hardware")]
        crate::PeripheralCommands::SetupUnoQ { host } => {
            uno_q_setup::setup_uno_q_bridge(host.as_deref())?;
        }
        #[cfg(not(feature = "hardware"))]
        crate::PeripheralCommands::SetupUnoQ { .. } => {
            println!("Uno Q setup requires the 'hardware' feature.");
            println!("Build with: cargo build --features hardware");
        }
        #[cfg(feature = "hardware")]
        crate::PeripheralCommands::FlashNucleo => {
            nucleo_flash::flash_nucleo_firmware()?;
        }
        #[cfg(not(feature = "hardware"))]
        crate::PeripheralCommands::FlashNucleo => {
            println!("Nucleo flash requires the 'hardware' feature.");
            println!("Build with: cargo build --features hardware");
        }
    }
    Ok(())
}

fn handle_list_command(config: &Config) {
    let boards = list_configured_boards(&config.peripherals);
    if boards.is_empty() {
        print_no_peripherals_help();
        return;
    }

    println!("Configured peripherals:");
    for board in boards {
        let path = board.path.as_deref().unwrap_or("(native)");
        println!("  {}  {}  {}", board.board, board.transport, path);
    }
}

fn print_no_peripherals_help() {
    println!("No peripherals configured.");
    println!();
    println!("Add one with: corvus peripheral add <board> <path>");
    println!("  Example: corvus peripheral add nucleo-f401re /dev/ttyACM0");
    println!();
    println!("Or add to config.toml:");
    println!("  [peripherals]");
    println!("  enabled = true");
    println!();
    println!("  [[peripherals.boards]]");
    println!("  board = \"nucleo-f401re\"");
    println!("  transport = \"serial\"");
    println!("  path = \"/dev/ttyACM0\"");
}

fn handle_add_command(board: String, path: String) -> Result<()> {
    let board = board.trim().to_string();
    if board.is_empty() {
        anyhow::bail!("Peripheral board name cannot be empty");
    }
    let path = path.trim().to_string();
    if path.is_empty() {
        anyhow::bail!("Peripheral path cannot be empty");
    }
    let transport = if path == "native" { "native" } else { "serial" };
    let path_opt = if path == "native" {
        None
    } else {
        Some(path.clone())
    };

    let mut cfg = Config::load_or_init()?;
    cfg.peripherals.enabled = true;

    if cfg
        .peripherals
        .boards
        .iter()
        .any(|entry| entry.board == board && entry.path.as_deref() == path_opt.as_deref())
    {
        println!("Board {} at {:?} already configured.", board, path_opt);
        return Ok(());
    }

    cfg.peripherals.boards.push(PeripheralBoardConfig {
        board: board.clone(),
        transport: transport.to_string(),
        path: path_opt,
        baud: 115_200,
    });
    cfg.save()?;
    println!("Added {} at {}. Restart daemon to apply.", board, path);
    Ok(())
}

/// Create and connect peripherals from config, returning their tools.
/// Returns empty vec if peripherals disabled or hardware feature off.
#[cfg(feature = "hardware")]
pub async fn create_peripheral_tools(config: &PeripheralsConfig) -> Result<Vec<Box<dyn Tool>>> {
    if !config.enabled || config.boards.is_empty() {
        return Ok(Vec::new());
    }

    let mut tools: Vec<Box<dyn Tool>> = Vec::new();
    let mut serial_transports: Vec<(String, std::sync::Arc<serial::SerialTransport>)> = Vec::new();
    let mut board_names: Vec<String> = Vec::new();

    for board in &config.boards {
        if let Err(reason) = validate_board_config(board) {
            tracing::warn!(board = %board.board, "Skipping peripheral config: {reason}");
            continue;
        }
        board_names.push(board.board.clone());
        if try_add_uno_q_bridge_tools(board, &mut tools) {
            continue;
        }

        if try_connect_native_rpi(board, &mut tools).await {
            continue;
        }

        connect_serial_board(board, &mut tools, &mut serial_transports).await;
    }

    // Phase B: Add hardware tools when any boards configured
    if !board_names.is_empty() {
        tools.push(Box::new(HardwareMemoryMapTool::new(board_names.clone())));
        tools.push(Box::new(crate::tools::HardwareBoardInfoTool::new(
            board_names.clone(),
        )));
        tools.push(Box::new(crate::tools::HardwareMemoryReadTool::new(
            board_names,
        )));
    }

    // Phase C: Add hardware_capabilities tool when any serial boards
    if !serial_transports.is_empty() {
        tools.push(Box::new(capabilities_tool::HardwareCapabilitiesTool::new(
            serial_transports,
        )));
    }

    Ok(tools)
}

fn validate_board_config(board: &PeripheralBoardConfig) -> Result<(), String> {
    let board_name = board.board.trim();
    if board_name.is_empty() {
        return Err("board name is empty".to_string());
    }

    match board.transport.as_str() {
        "serial" => {
            if board
                .path
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            {
                return Err("serial transport requires a path".to_string());
            }
        }
        "native" | "bridge" => {}
        other => {
            return Err(format!(
                "unsupported transport '{other}' (use: serial, native, bridge)"
            ));
        }
    }

    Ok(())
}

#[cfg(feature = "hardware")]
fn try_add_uno_q_bridge_tools(
    board: &PeripheralBoardConfig,
    tools: &mut Vec<Box<dyn Tool>>,
) -> bool {
    if board.transport == "bridge" && (board.board == "arduino-uno-q" || board.board == "uno-q") {
        tools.push(Box::new(uno_q_bridge::UnoQGpioReadTool));
        tools.push(Box::new(uno_q_bridge::UnoQGpioWriteTool));
        tracing::info!(board = %board.board, "Uno Q Bridge GPIO tools added");
        return true;
    }
    false
}

#[cfg(feature = "hardware")]
#[cfg(all(feature = "peripheral-rpi", target_os = "linux"))]
async fn try_connect_native_rpi(
    board: &PeripheralBoardConfig,
    tools: &mut Vec<Box<dyn Tool>>,
) -> bool {
    if board.transport != "native" || (board.board != "rpi-gpio" && board.board != "raspberry-pi") {
        return false;
    }

    match rpi::RpiGpioPeripheral::connect_from_config(board).await {
        Ok(peripheral) => {
            tools.extend(peripheral.tools());
            tracing::info!(board = %board.board, "RPi GPIO peripheral connected");
        }
        Err(e) => {
            tracing::warn!("Failed to connect RPi GPIO {}: {}", board.board, e);
        }
    }
    true
}

#[cfg(feature = "hardware")]
#[cfg(not(all(feature = "peripheral-rpi", target_os = "linux")))]
#[allow(clippy::unused_async)]
async fn try_connect_native_rpi(
    _board: &PeripheralBoardConfig,
    _tools: &mut Vec<Box<dyn Tool>>,
) -> bool {
    false
}

#[cfg(feature = "hardware")]
async fn connect_serial_board(
    board: &PeripheralBoardConfig,
    tools: &mut Vec<Box<dyn Tool>>,
    serial_transports: &mut Vec<(String, std::sync::Arc<serial::SerialTransport>)>,
) {
    if board.transport != "serial" {
        return;
    }
    let Some(path) = board.path.as_deref() else {
        tracing::warn!("Skipping serial board {}: no path", board.board);
        return;
    };

    match serial::SerialPeripheral::connect(board).await {
        Ok(peripheral) => {
            let mut connected = peripheral;
            if connected.connect().await.is_err() {
                tracing::warn!(
                    "Peripheral {} connect warning (continuing)",
                    connected.name()
                );
            }

            serial_transports.push((board.board.clone(), connected.transport()));
            tools.extend(connected.tools());

            if board.board == "arduino-uno" {
                tools.push(Box::new(arduino_upload::ArduinoUploadTool::new(
                    path.to_string(),
                )));
                tracing::info!("Arduino upload tool added (port: {})", path);
            }
            tracing::info!(board = %board.board, "Serial peripheral connected");
        }
        Err(e) => {
            tracing::warn!("Failed to connect {}: {}", board.board, e);
        }
    }
}

#[cfg(not(feature = "hardware"))]
pub async fn create_peripheral_tools(_config: &PeripheralsConfig) -> Result<Vec<Box<dyn Tool>>> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board(board: &str, transport: &str, path: Option<&str>) -> PeripheralBoardConfig {
        PeripheralBoardConfig {
            board: board.to_string(),
            transport: transport.to_string(),
            path: path.map(str::to_string),
            baud: 115_200,
        }
    }

    #[test]
    fn list_configured_boards_returns_empty_when_disabled() {
        let config = PeripheralsConfig {
            enabled: false,
            boards: vec![board("nucleo-f401re", "serial", Some("/dev/ttyACM0"))],
            datasheet_dir: None,
        };

        assert!(list_configured_boards(&config).is_empty());
    }

    #[test]
    fn list_configured_boards_returns_all_boards_when_enabled() {
        let config = PeripheralsConfig {
            enabled: true,
            boards: vec![
                board("nucleo-f401re", "serial", Some("/dev/ttyACM0")),
                board("arduino-uno-q", "bridge", None),
            ],
            datasheet_dir: None,
        };

        let boards = list_configured_boards(&config);
        assert_eq!(boards.len(), 2);
        assert_eq!(boards[0].board, "nucleo-f401re");
        assert_eq!(boards[1].transport, "bridge");
    }

    #[test]
    fn validate_board_config_rejects_invalid_entries() {
        assert_eq!(
            validate_board_config(&board("", "serial", Some("/dev/ttyACM0"))).unwrap_err(),
            "board name is empty"
        );
        assert_eq!(
            validate_board_config(&board("uno", "serial", Some("   "))).unwrap_err(),
            "serial transport requires a path"
        );
        assert_eq!(
            validate_board_config(&board("uno", "serial", None)).unwrap_err(),
            "serial transport requires a path"
        );
        assert_eq!(
            validate_board_config(&board("uno", "bluetooth", None)).unwrap_err(),
            "unsupported transport 'bluetooth' (use: serial, native, bridge)"
        );
    }

    #[test]
    fn validate_board_config_accepts_supported_transports() {
        assert!(validate_board_config(&board("uno", "native", None)).is_ok());
        assert!(validate_board_config(&board("uno", "bridge", None)).is_ok());
        assert!(validate_board_config(&board("uno", "serial", Some("/dev/ttyUSB0"))).is_ok());
    }

    #[cfg(feature = "hardware")]
    #[test]
    fn try_add_uno_q_bridge_tools_only_for_bridge_transport() {
        let mut tools: Vec<Box<dyn Tool>> = Vec::new();

        assert!(try_add_uno_q_bridge_tools(
            &board("arduino-uno-q", "bridge", None),
            &mut tools,
        ));
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name(), "gpio_read");
        assert_eq!(tools[1].name(), "gpio_write");

        let mut tools: Vec<Box<dyn Tool>> = Vec::new();
        assert!(!try_add_uno_q_bridge_tools(
            &board("arduino-uno-q", "serial", Some("/dev/ttyACM0")),
            &mut tools,
        ));
        assert!(tools.is_empty());
    }
}
