//! Device introspection — correlate serial path with USB device info.

use super::discover;
use super::registry;
use anyhow::Result;

/// Result of introspecting a device by path.
#[derive(Debug, Clone)]
pub struct IntrospectResult {
    pub path: String,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub board_name: Option<String>,
    pub architecture: Option<String>,
    pub memory_map_note: String,
}

/// Introspect a device by its serial path (e.g. /dev/ttyACM0, /dev/tty.usbmodem*).
/// Attempts to correlate with USB devices from discovery.
#[cfg(feature = "hardware")]
pub fn introspect_device(path: &str) -> Result<IntrospectResult> {
    let devices = discover::list_usb_devices()?;

    // Try to correlate path with a discovered device.
    // On Linux, /dev/ttyACM0 corresponds to a CDC-ACM device; we may have multiple.
    // Best-effort: if we have exactly one CDC-like device, use it. Otherwise unknown.
    let matched = if devices.len() == 1 {
        devices.first().cloned()
    } else if devices.is_empty() {
        None
    } else {
        // Multiple devices: try to match by path. On Linux we could use sysfs;
        // for stub, pick first known board or first device.
        devices
            .iter()
            .find(|d| d.board_name.is_some())
            .cloned()
            .or_else(|| devices.first().cloned())
    };

    let (vid, pid, board_name, architecture) = match matched {
        Some(d) => (Some(d.vid), Some(d.pid), d.board_name, d.architecture),
        None => (None, None, None, None),
    };

    let board_info = vid.and_then(|v| pid.and_then(|p| registry::lookup_board(v, p)));
    let architecture =
        architecture.or_else(|| board_info.and_then(|b| b.architecture.map(String::from)));
    let board_name = board_name.or_else(|| board_info.map(|b| b.name.to_string()));

    let memory_map_note = memory_map_for_board(board_name.as_deref());

    Ok(IntrospectResult {
        path: path.to_string(),
        vid,
        pid,
        board_name,
        architecture,
        memory_map_note,
    })
}

/// Get memory map: via probe-rs when probe feature on and Nucleo, else static or stub.
#[cfg(feature = "hardware")]
fn memory_map_for_board(board_name: Option<&str>) -> String {
    #[cfg(feature = "probe")]
    if let Some(board) = board_name {
        let chip = match board {
            "nucleo-f401re" => "STM32F401RETx",
            "nucleo-f411re" => "STM32F411RETx",
            _ => return "Build with --features probe for live memory map (Nucleo)".to_string(),
        };
        match probe_memory_map(chip) {
            Ok(s) => return s,
            Err(_) => return format!("probe-rs attach failed (chip {}). Connect via USB.", chip),
        }
    }

    #[cfg(not(feature = "probe"))]
    let _ = board_name;

    "Build with --features probe for live memory map via USB".to_string()
}

#[cfg(all(feature = "hardware", feature = "probe"))]
fn probe_memory_map(chip: &str) -> anyhow::Result<String> {
    use probe_rs::config::MemoryRegion;
    use probe_rs::{Session, SessionConfig};
    use std::fmt::Write;

    let session = Session::auto_attach(chip, SessionConfig::default())
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let target = session.target();
    let mut out = String::new();
    for region in &target.memory_map {
        match region {
            MemoryRegion::Ram(ram) => {
                let (start, end) = (ram.range.start, ram.range.end);
                let _ = writeln!(
                    out,
                    "RAM: 0x{:08X} - 0x{:08X} ({} KB)",
                    start,
                    end,
                    (end - start) / 1024
                );
            }
            MemoryRegion::Nvm(flash) => {
                let (start, end) = (flash.range.start, flash.range.end);
                let _ = writeln!(
                    out,
                    "Flash: 0x{:08X} - 0x{:08X} ({} KB)",
                    start,
                    end,
                    (end - start) / 1024
                );
            }
            MemoryRegion::Generic(_) => {}
        }
    }
    if out.is_empty() {
        out = "Could not read memory regions".to_string();
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "hardware", not(feature = "probe")))]
    #[test]
    fn memory_map_for_board_uses_static_probe_guidance_without_probe_feature() {
        let note = memory_map_for_board(Some("nucleo-f401re"));
        assert!(note.contains("Build with --features probe"));

        let unknown = memory_map_for_board(Some("custom-board"));
        assert!(unknown.contains("Build with --features probe"));
    }

    #[test]
    fn introspect_result_keeps_supplied_path() {
        let result = IntrospectResult {
            path: "/dev/ttyACM0".to_string(),
            vid: Some(0x0483),
            pid: Some(0x5740),
            board_name: Some("nucleo-f401re".to_string()),
            architecture: Some("arm".to_string()),
            memory_map_note: "note".to_string(),
        };

        assert_eq!(result.path, "/dev/ttyACM0");
        assert_eq!(result.board_name.as_deref(), Some("nucleo-f401re"));
    }
}
