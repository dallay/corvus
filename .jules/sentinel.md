# Sentinel's Security Journal

## 2026-02-21 - [Permission Race Condition in Secret Storage]
**Vulnerability:** A permission race condition existed in `SecretStore::load_or_create_key` where the secret key file was created with default permissions and then restricted using `fs::set_permissions`. This created a small window where other users could potentially read the secret key.
**Learning:** Standard `fs::write` or `File::create` on Unix uses the process umask for initial permissions. For sensitive files, atomic creation with restrictive permissions is necessary.
**Prevention:** Use `std::fs::OpenOptions` with the Unix-specific `.mode(0o600)` extension to ensure the file is created with owner-only access from the start.

## 2026-02-21 - [Incomplete Output Capture in Testable Subprocesses]
**Vulnerability:** `run_job_command_with_timeout` was spawning shell commands without explicitly piping stdout/stderr, causing `wait_with_output` to return empty buffers while the output leaked to the parent's console.
**Learning:** In Tokio's `Command`, output must be explicitly piped if it needs to be captured and verified by tests or processed by the application.
**Prevention:** Always use `.stdout(Stdio::piped()).stderr(Stdio::piped())` when the output of a spawned process needs to be captured.
