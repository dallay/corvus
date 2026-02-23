use crate::config::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const SERVICE_LABEL: &str = "com.corvus.daemon";
const WINDOWS_TASK_NAME: &str = "Corvus Daemon";

fn windows_task_name() -> &'static str {
    WINDOWS_TASK_NAME
}

pub fn handle_command(command: &crate::ServiceCommands, config: &Config) -> Result<()> {
    match command {
        crate::ServiceCommands::Install { linger } => install(config, *linger),
        crate::ServiceCommands::Start => start(config),
        crate::ServiceCommands::Restart => restart(config),
        crate::ServiceCommands::Stop => stop(config),
        crate::ServiceCommands::Status => status(config),
        crate::ServiceCommands::Uninstall => uninstall(config),
    }
}

fn install(config: &Config, linger: crate::ServiceLingerMode) -> Result<()> {
    if cfg!(target_os = "macos") {
        maybe_warn_unsupported_linger_mode(linger);
        install_macos(config)
    } else if cfg!(target_os = "linux") {
        install_linux(config)?;
        apply_linux_linger_mode(linger)
    } else if cfg!(target_os = "windows") {
        maybe_warn_unsupported_linger_mode(linger);
        install_windows(config)
    } else {
        anyhow::bail!("Service management is supported on macOS and Linux only");
    }
}

fn restart(config: &Config) -> Result<()> {
    if cfg!(target_os = "linux") {
        if run_checked(Command::new("systemctl").args(["--user", "restart", "corvus.service"]))
            .is_ok()
        {
            println!("✅ Service restarted");
            return Ok(());
        }
    }

    stop(config)?;
    start(config)
}

fn start(config: &Config) -> Result<()> {
    if cfg!(target_os = "macos") {
        let plist = macos_service_file()?;
        run_checked(Command::new("launchctl").arg("load").arg("-w").arg(&plist))?;
        run_checked(Command::new("launchctl").arg("start").arg(SERVICE_LABEL))?;
        println!("✅ Service started");
        Ok(())
    } else if cfg!(target_os = "linux") {
        run_checked(Command::new("systemctl").args(["--user", "daemon-reload"]))?;
        run_checked(Command::new("systemctl").args(["--user", "start", "corvus.service"]))?;
        println!("✅ Service started");
        Ok(())
    } else if cfg!(target_os = "windows") {
        let _ = config;
        run_checked(Command::new("schtasks").args(["/Run", "/TN", windows_task_name()]))?;
        println!("✅ Service started");
        Ok(())
    } else {
        let _ = config;
        anyhow::bail!("Service management is supported on macOS and Linux only")
    }
}

fn stop(config: &Config) -> Result<()> {
    if cfg!(target_os = "macos") {
        let plist = macos_service_file()?;
        let _ = run_checked(Command::new("launchctl").arg("stop").arg(SERVICE_LABEL));
        let _ = run_checked(
            Command::new("launchctl")
                .arg("unload")
                .arg("-w")
                .arg(&plist),
        );
        println!("✅ Service stopped");
        Ok(())
    } else if cfg!(target_os = "linux") {
        let _ = run_checked(Command::new("systemctl").args(["--user", "stop", "corvus.service"]));
        println!("✅ Service stopped");
        Ok(())
    } else if cfg!(target_os = "windows") {
        let _ = config;
        let task_name = windows_task_name();
        let _ = run_checked(Command::new("schtasks").args(["/End", "/TN", task_name]));
        println!("✅ Service stopped");
        Ok(())
    } else {
        let _ = config;
        anyhow::bail!("Service management is supported on macOS and Linux only")
    }
}

fn status(config: &Config) -> Result<()> {
    if cfg!(target_os = "macos") {
        let out = run_capture(Command::new("launchctl").arg("list"))?;
        let running = out.lines().any(|line| line.contains(SERVICE_LABEL));
        println!(
            "Service: {}",
            if running {
                "✅ running/loaded"
            } else {
                "❌ not loaded"
            }
        );
        println!("Unit: {}", macos_service_file()?.display());
        return Ok(());
    }

    if cfg!(target_os = "linux") {
        let out =
            run_capture(Command::new("systemctl").args(["--user", "is-active", "corvus.service"]))
                .unwrap_or_else(|_| "unknown".into());
        println!("Service state: {}", out.trim());
        match linux_linger_state() {
            Ok(Some(enabled)) => println!(
                "Linger: {}",
                if enabled {
                    "enabled (keeps service alive without login)"
                } else {
                    "disabled (service starts after user login)"
                }
            ),
            Ok(None) => println!("Linger: unknown"),
            Err(e) => println!("Linger: unavailable ({e})"),
        }
        println!("Unit: {}", linux_service_file(config)?.display());
        return Ok(());
    }

    if cfg!(target_os = "windows") {
        let _ = config;
        let task_name = windows_task_name();
        let out =
            run_capture(Command::new("schtasks").args(["/Query", "/TN", task_name, "/FO", "LIST"]));
        match out {
            Ok(text) => {
                let running = text.contains("Running");
                println!(
                    "Service: {}",
                    if running {
                        "✅ running"
                    } else {
                        "❌ not running"
                    }
                );
                println!("Task: {}", task_name);
            }
            Err(_) => {
                println!("Service: ❌ not installed");
            }
        }
        return Ok(());
    }

    anyhow::bail!("Service management is supported on macOS and Linux only")
}

fn maybe_warn_unsupported_linger_mode(linger: crate::ServiceLingerMode) {
    if !matches!(linger, crate::ServiceLingerMode::Keep) {
        eprintln!(
            "⚠️  --linger applies only to Linux user services; ignoring requested mode on this OS."
        );
    }
}

fn current_username() -> Result<String> {
    let env_username = std::env::var("USER")
        .ok()
        .or_else(|| std::env::var("LOGNAME").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if let Some(username) = env_username {
        return Ok(username);
    }

    let fallback_username = Command::new("whoami")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    fallback_username.context("Could not resolve current username (USER/LOGNAME)")
}

fn apply_linux_linger_mode(linger: crate::ServiceLingerMode) -> Result<()> {
    match linger {
        crate::ServiceLingerMode::Keep => Ok(()),
        crate::ServiceLingerMode::On => {
            let user = current_username()?;
            run_checked(Command::new("loginctl").args(["enable-linger", &user]))?;
            println!("✅ Enabled linger for user '{user}'");
            Ok(())
        }
        crate::ServiceLingerMode::Off => {
            let user = current_username()?;
            run_checked(Command::new("loginctl").args(["disable-linger", &user]))?;
            println!("✅ Disabled linger for user '{user}'");
            Ok(())
        }
    }
}

fn linux_linger_state() -> Result<Option<bool>> {
    if !cfg!(target_os = "linux") {
        return Ok(None);
    }

    let user = current_username()?;
    let output = Command::new("loginctl")
        .args(["show-user", &user, "-p", "Linger"])
        .output()
        .context("Failed to spawn command")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {}", stderr.trim());
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let value = raw
        .trim()
        .split_once('=')
        .map_or(raw.trim(), |(_, rhs)| rhs.trim());
    if value.is_empty() {
        return Ok(None);
    }

    Ok(Some(value.eq_ignore_ascii_case("yes")))
}

fn uninstall(config: &Config) -> Result<()> {
    stop(config)?;

    if cfg!(target_os = "macos") {
        let file = macos_service_file()?;
        if file.exists() {
            fs::remove_file(&file)
                .with_context(|| format!("Failed to remove {}", file.display()))?;
        }
        println!("✅ Service uninstalled ({})", file.display());
        return Ok(());
    }

    if cfg!(target_os = "linux") {
        let file = linux_service_file(config)?;
        if file.exists() {
            fs::remove_file(&file)
                .with_context(|| format!("Failed to remove {}", file.display()))?;
        }
        let _ = run_checked(Command::new("systemctl").args(["--user", "daemon-reload"]));
        println!("✅ Service uninstalled ({})", file.display());
        return Ok(());
    }

    if cfg!(target_os = "windows") {
        let task_name = windows_task_name();
        let _ = run_checked(Command::new("schtasks").args(["/Delete", "/TN", task_name, "/F"]));
        // Remove the wrapper script
        let wrapper = config
            .config_path
            .parent()
            .map_or_else(|| PathBuf::from("."), PathBuf::from)
            .join("logs")
            .join("corvus-daemon.cmd");
        if wrapper.exists() {
            fs::remove_file(&wrapper).ok();
        }
        println!("✅ Service uninstalled");
        return Ok(());
    }

    anyhow::bail!("Service management is supported on macOS and Linux only")
}

fn install_macos(config: &Config) -> Result<()> {
    let file = macos_service_file()?;
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }

    let exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let logs_dir = config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("logs");
    fs::create_dir_all(&logs_dir)?;

    let stdout = logs_dir.join("daemon.stdout.log");
    let stderr = logs_dir.join("daemon.stderr.log");

    let plist = format!(
        r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
    <string>daemon</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>{stdout}</string>
  <key>StandardErrorPath</key>
  <string>{stderr}</string>
</dict>
</plist>
"#,
        label = SERVICE_LABEL,
        exe = xml_escape(&exe.display().to_string()),
        stdout = xml_escape(&stdout.display().to_string()),
        stderr = xml_escape(&stderr.display().to_string())
    );

    fs::write(&file, plist)?;
    println!("✅ Installed launchd service: {}", file.display());
    println!("   Start with: corvus service start");
    Ok(())
}

fn install_linux(config: &Config) -> Result<()> {
    let file = linux_service_file(config)?;
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }

    let exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let unit = format!(
        "[Unit]\nDescription=Corvus daemon\nAfter=network.target\n\n[Service]\nType=simple\nExecStart={} daemon\nRestart=always\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n",
        exe.display()
    );

    fs::write(&file, unit)?;
    let _ = run_checked(Command::new("systemctl").args(["--user", "daemon-reload"]));
    let _ = run_checked(Command::new("systemctl").args(["--user", "enable", "corvus.service"]));
    println!("✅ Installed systemd user service: {}", file.display());
    println!("   Start with: corvus service start");
    Ok(())
}

fn install_windows(config: &Config) -> Result<()> {
    let exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let logs_dir = config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("logs");
    fs::create_dir_all(&logs_dir)?;

    // Create a wrapper script that redirects output to log files
    let wrapper = logs_dir.join("corvus-daemon.cmd");
    let stdout_log = logs_dir.join("daemon.stdout.log");
    let stderr_log = logs_dir.join("daemon.stderr.log");

    let wrapper_content = format!(
        "@echo off\r\n\"{}\" daemon >>\"{}\" 2>>\"{}\"",
        exe.display(),
        stdout_log.display(),
        stderr_log.display()
    );
    fs::write(&wrapper, &wrapper_content)?;

    let task_name = windows_task_name();

    // Remove any existing task first (ignore errors if it doesn't exist)
    let _ = Command::new("schtasks")
        .args(["/Delete", "/TN", task_name, "/F"])
        .output();

    run_checked(Command::new("schtasks").args([
        "/Create",
        "/TN",
        task_name,
        "/SC",
        "ONLOGON",
        "/TR",
        &format!("\"{}\"", wrapper.display()),
        "/RL",
        "HIGHEST",
        "/F",
    ]))?;

    println!("✅ Installed Windows scheduled task: {}", task_name);
    println!("   Wrapper: {}", wrapper.display());
    println!("   Logs: {}", logs_dir.display());
    println!("   Start with: corvus service start");
    Ok(())
}

fn macos_service_file() -> Result<PathBuf> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    Ok(home
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{SERVICE_LABEL}.plist")))
}

fn linux_service_file(config: &Config) -> Result<PathBuf> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    let _ = config;
    Ok(home
        .join(".config")
        .join("systemd")
        .join("user")
        .join("corvus.service"))
}

fn run_checked(command: &mut Command) -> Result<()> {
    let output = command.output().context("Failed to spawn command")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {}", stderr.trim());
    }
    Ok(())
}

fn run_capture(command: &mut Command) -> Result<String> {
    let output = command.output().context("Failed to spawn command")?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stderr).to_string();
    }
    Ok(text)
}

fn xml_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn set_env(key: &str, value: &str) {
        // SAFETY: tests take ENV_LOCK to serialize process-wide env mutation.
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_env(key: &str) {
        // SAFETY: tests take ENV_LOCK to serialize process-wide env mutation.
        unsafe { std::env::remove_var(key) };
    }

    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            set_env(key, value);
            Self { key, original }
        }

        fn remove(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            remove_env(key);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original {
                set_env(self.key, value);
            } else {
                remove_env(self.key);
            }
        }
    }

    #[test]
    fn xml_escape_escapes_reserved_chars() {
        let escaped = xml_escape("<&>\"' and text");
        assert_eq!(escaped, "&lt;&amp;&gt;&quot;&apos; and text");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn run_capture_reads_stdout() {
        let out = run_capture(Command::new("sh").args(["-lc", "echo hello"]))
            .expect("stdout capture should succeed");
        assert_eq!(out.trim(), "hello");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn run_capture_falls_back_to_stderr() {
        let out = run_capture(Command::new("sh").args(["-lc", "echo warn 1>&2"]))
            .expect("stderr capture should succeed");
        assert_eq!(out.trim(), "warn");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn run_checked_errors_on_non_zero_status() {
        let err = run_checked(Command::new("sh").args(["-lc", "exit 17"]))
            .expect_err("non-zero exit should error");
        assert!(err.to_string().contains("Command failed"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn linux_service_file_has_expected_suffix() {
        let file = linux_service_file(&Config::default()).unwrap();
        let path = file.to_string_lossy();
        assert!(path.ends_with(".config/systemd/user/corvus.service"));
    }

    #[test]
    fn windows_task_name_is_constant() {
        assert_eq!(windows_task_name(), "Corvus Daemon");
    }

    #[test]
    fn current_username_prefers_user_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _user = EnvGuard::set("USER", "testuser");
        let _logname = EnvGuard::set("LOGNAME", "loguser");

        let result = current_username().unwrap();

        assert_eq!(result, "testuser");
    }

    #[test]
    fn current_username_uses_logname_when_user_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _user = EnvGuard::remove("USER");
        let _logname = EnvGuard::set("LOGNAME", "loguser");

        let result = current_username().unwrap();

        assert_eq!(result, "loguser");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_helpers_compile_and_are_callable() {
        let _ = apply_linux_linger_mode(crate::ServiceLingerMode::Keep);
        let _ = linux_linger_state();
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn linux_helpers_compile_on_non_linux() {
        let _apply: fn(crate::ServiceLingerMode) -> Result<()> = apply_linux_linger_mode;
        let _state: fn() -> Result<Option<bool>> = linux_linger_state;
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn run_capture_reads_stdout_windows() {
        let out = run_capture(Command::new("cmd").args(["/C", "echo hello"]))
            .expect("stdout capture should succeed");
        assert_eq!(out.trim(), "hello");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn run_checked_errors_on_non_zero_status_windows() {
        let err = run_checked(Command::new("cmd").args(["/C", "exit /b 17"]))
            .expect_err("non-zero exit should error");
        assert!(err.to_string().contains("Command failed"));
    }
}
