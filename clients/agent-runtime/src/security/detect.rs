//! Auto-detection of available security features

use crate::config::{SandboxBackend, SecurityConfig};
use crate::security::traits::Sandbox;
use anyhow::{anyhow, Result};
use std::sync::Arc;

/// Create a sandbox based on auto-detection or explicit config.
///
/// Returns `Err` when `config.sandbox.require == true` and no real
/// OS-level backend is available. Returns `Ok(NoopSandbox)` when
/// `require == false` and no backend is found.
pub fn create_sandbox(config: &SecurityConfig) -> Result<Arc<dyn Sandbox>> {
    let backend = &config.sandbox.backend;
    let require = config.sandbox.require;

    // If explicitly disabled or backend=None, return noop or error
    if matches!(backend, SandboxBackend::None) || config.sandbox.enabled == Some(false) {
        if require {
            return Err(anyhow!(
                "Sandbox required but configuration disables it \
                 (backend=none or enabled=false)"
            ));
        }
        return Ok(Arc::new(super::traits::NoopSandbox));
    }

    // If specific backend requested, try that
    match backend {
        SandboxBackend::Landlock => {
            #[cfg(feature = "sandbox-landlock")]
            {
                #[cfg(target_os = "linux")]
                {
                    if let Ok(sandbox) = super::landlock::LandlockSandbox::new() {
                        return Ok(Arc::new(sandbox));
                    }
                }
            }
            if require {
                return Err(anyhow!(
                    "Sandbox required but Landlock backend is not available \
                     on this platform"
                ));
            }
            tracing::warn!(
                "Landlock requested but not available, \
                 falling back to application-layer"
            );
            Ok(Arc::new(super::traits::NoopSandbox))
        }
        SandboxBackend::Firejail => {
            #[cfg(target_os = "linux")]
            {
                if let Ok(sandbox) = super::firejail::FirejailSandbox::new() {
                    return Ok(Arc::new(sandbox));
                }
            }
            if require {
                return Err(anyhow!(
                    "Sandbox required but Firejail backend is not available"
                ));
            }
            tracing::warn!(
                "Firejail requested but not available, \
                 falling back to application-layer"
            );
            Ok(Arc::new(super::traits::NoopSandbox))
        }
        SandboxBackend::Bubblewrap => {
            #[cfg(feature = "sandbox-bubblewrap")]
            {
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                {
                    if let Ok(sandbox) = super::bubblewrap::BubblewrapSandbox::new() {
                        return Ok(Arc::new(sandbox));
                    }
                }
            }
            if require {
                return Err(anyhow!(
                    "Sandbox required but Bubblewrap backend is not available"
                ));
            }
            tracing::warn!(
                "Bubblewrap requested but not available, \
                 falling back to application-layer"
            );
            Ok(Arc::new(super::traits::NoopSandbox))
        }
        SandboxBackend::Docker => {
            if let Ok(sandbox) = super::docker::DockerSandbox::new() {
                return Ok(Arc::new(sandbox));
            }
            if require {
                return Err(anyhow!(
                    "Sandbox required but Docker backend is not available"
                ));
            }
            tracing::warn!(
                "Docker requested but not available, \
                 falling back to application-layer"
            );
            Ok(Arc::new(super::traits::NoopSandbox))
        }
        SandboxBackend::Auto | SandboxBackend::None => {
            // Auto-detect best available
            detect_best_sandbox(require)
        }
    }
}

/// Auto-detect the best available sandbox.
///
/// When `require` is true and no backend is found, returns `Err`.
fn detect_best_sandbox(require: bool) -> Result<Arc<dyn Sandbox>> {
    #[cfg(target_os = "linux")]
    {
        // Try Landlock first (native, no dependencies)
        #[cfg(feature = "sandbox-landlock")]
        {
            if let Ok(sandbox) = super::landlock::LandlockSandbox::probe() {
                tracing::info!("Landlock sandbox enabled (Linux kernel 5.13+)");
                return Ok(Arc::new(sandbox));
            }
        }

        // Try Firejail second (user-space tool)
        if let Ok(sandbox) = super::firejail::FirejailSandbox::probe() {
            tracing::info!("Firejail sandbox enabled");
            return Ok(Arc::new(sandbox));
        }

        // Try Bubblewrap third (common on Linux via Flatpak)
        #[cfg(feature = "sandbox-bubblewrap")]
        {
            if let Ok(sandbox) = super::bubblewrap::BubblewrapSandbox::probe() {
                tracing::info!("Auto-detected sandbox backend: bubblewrap");
                return Ok(Arc::new(sandbox));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Try Bubblewrap on macOS
        #[cfg(feature = "sandbox-bubblewrap")]
        {
            if let Ok(sandbox) = super::bubblewrap::BubblewrapSandbox::probe() {
                tracing::info!("Bubblewrap sandbox enabled");
                return Ok(Arc::new(sandbox));
            }
        }
    }

    // Docker is heavy but works everywhere if docker is installed
    if let Ok(sandbox) = super::docker::DockerSandbox::probe() {
        tracing::info!("Docker sandbox enabled");
        return Ok(Arc::new(sandbox));
    }

    // No backend found
    if require {
        return Err(anyhow!(
            "Sandbox required but no OS-level backend is available \
             (tried all supported backends for this platform)"
        ));
    }

    tracing::info!("No sandbox backend available, using application-layer security");
    Ok(Arc::new(super::traits::NoopSandbox))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SandboxConfig, SecurityConfig};

    #[test]
    fn detect_best_sandbox_returns_something() {
        let sandbox = detect_best_sandbox(false).unwrap();
        // Should always return at least NoopSandbox
        assert!(sandbox.is_available());
    }

    #[test]
    fn explicit_none_returns_noop() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(false),
                backend: SandboxBackend::None,
                require: false,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let sandbox = create_sandbox(&config).unwrap();
        assert_eq!(sandbox.name(), "none");
    }

    #[test]
    fn auto_mode_detects_something() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: None, // Auto-detect
                backend: SandboxBackend::Auto,
                require: false,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let sandbox = create_sandbox(&config).unwrap();
        // Should return some sandbox (at least NoopSandbox)
        assert!(sandbox.is_available());
    }

    #[test]
    fn explicit_disabled_returns_noop_regardless_of_backend() {
        // Even if a specific backend is set, enabled=false should override
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(false),
                backend: SandboxBackend::Docker,
                require: false,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let sandbox = create_sandbox(&config).unwrap();
        assert_eq!(sandbox.name(), "none");
    }

    #[test]
    fn none_backend_returns_noop() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(true),
                backend: SandboxBackend::None,
                require: false,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let sandbox = create_sandbox(&config).unwrap();
        assert_eq!(sandbox.name(), "none");
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn landlock_backend_falls_back_on_non_linux() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(true),
                backend: SandboxBackend::Landlock,
                require: false,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let sandbox = create_sandbox(&config).unwrap();
        // On macOS / non-Linux, Landlock is unavailable — falls back to noop
        assert_eq!(sandbox.name(), "none");
    }

    #[test]
    fn firejail_backend_falls_back_gracefully() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(true),
                backend: SandboxBackend::Firejail,
                require: false,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let sandbox = create_sandbox(&config).unwrap();
        // On most CI/test environments, firejail is not installed
        let name = sandbox.name();
        assert!(
            name == "firejail" || name == "none",
            "expected 'firejail' or 'none', got '{name}'"
        );
    }

    #[test]
    fn bubblewrap_backend_falls_back_gracefully() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(true),
                backend: SandboxBackend::Bubblewrap,
                require: false,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let sandbox = create_sandbox(&config).unwrap();
        let name = sandbox.name();
        assert!(
            name == "bubblewrap" || name == "none",
            "expected 'bubblewrap' or 'none', got '{name}'"
        );
    }

    #[test]
    fn docker_backend_falls_back_gracefully() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(true),
                backend: SandboxBackend::Docker,
                require: false,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let sandbox = create_sandbox(&config).unwrap();
        let name = sandbox.name();
        assert!(
            name == "docker" || name == "none",
            "expected 'docker' or 'none', got '{name}'"
        );
    }

    #[test]
    fn auto_backend_with_enabled_true_detects_something() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(true),
                backend: SandboxBackend::Auto,
                require: false,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let sandbox = create_sandbox(&config).unwrap();
        assert!(sandbox.is_available());
    }

    #[test]
    fn detect_best_sandbox_name_is_non_empty() {
        let sandbox = detect_best_sandbox(false).unwrap();
        assert!(!sandbox.name().is_empty());
    }

    #[test]
    fn default_security_config_produces_working_sandbox() {
        let config = SecurityConfig::default();
        let sandbox = create_sandbox(&config).unwrap();
        assert!(sandbox.is_available());
        assert!(!sandbox.name().is_empty());
    }

    // ── Fail-closed (require=true) tests (T2) ──────────────

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn require_landlock_on_non_linux_returns_error() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(true),
                backend: SandboxBackend::Landlock,
                require: true,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let result = create_sandbox(&config);
        assert!(result.is_err(), "require+unavailable must return Err");
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("Landlock"),
            "error must mention Landlock: {msg}"
        );
    }

    #[test]
    fn require_none_backend_returns_error() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(true),
                backend: SandboxBackend::None,
                require: true,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let result = create_sandbox(&config);
        assert!(result.is_err(), "require+backend=none must return Err");
    }

    #[test]
    fn require_disabled_returns_error() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(false),
                backend: SandboxBackend::Auto,
                require: true,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let result = create_sandbox(&config);
        assert!(result.is_err(), "require+enabled=false must return Err");
    }

    #[test]
    fn require_auto_no_backend_returns_error_or_ok() {
        // On macOS without Docker, this should error.
        // On systems with Docker, it should succeed.
        // Either way, it must not panic.
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(true),
                backend: SandboxBackend::Auto,
                require: true,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let result = create_sandbox(&config);
        match &result {
            Ok(sandbox) => {
                assert_ne!(
                    sandbox.name(),
                    "none",
                    "require=true must not return NoopSandbox"
                );
            }
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("no OS-level backend"),
                    "error must explain no backend found: {msg}"
                );
            }
        }
    }

    #[test]
    fn detect_best_sandbox_require_true_no_noop() {
        let result = detect_best_sandbox(true);
        match &result {
            Ok(sandbox) => {
                assert_ne!(
                    sandbox.name(),
                    "none",
                    "require=true must never return NoopSandbox"
                );
            }
            Err(_) => {
                // Expected on systems without any sandbox backend
            }
        }
    }
}
