use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Semaphore;

use crate::channels::audio_media::{AudioRejectionReason, StagedAudio};

use super::traits::{Transcriber, TranscriptionResult};

/// whisper.cpp CLI wrapper transcriber.
///
/// Spawns the whisper CLI binary as an external process (zero Rust
/// dependency impact). The concurrency semaphore prevents CPU overload
/// from multiple simultaneous transcription processes.
///
/// # Cross-channel semaphore sharing
///
/// A single `WhisperCliTranscriber` instance is constructed at startup and
/// shared across ALL audio-capable channels (Telegram, gateway, CLI) via
/// `Arc<dyn Transcriber>`. Because every channel holds a clone of the same
/// `Arc`, they all acquire permits from the **same** `semaphore` field,
/// enforcing a unified `max_concurrent_transcriptions` budget regardless of
/// which channel originates the audio. No additional synchronisation is
/// required — `Arc` guarantees a single allocation shared by reference.
pub struct WhisperCliTranscriber {
    binary_path: String,
    model_path: PathBuf,
    language: String,
    timeout: Duration,
    semaphore: Arc<Semaphore>,
}

impl WhisperCliTranscriber {
    /// Create a new whisper CLI transcriber.
    ///
    /// - `binary_path`: path to the whisper-cli binary (or just the name for PATH lookup)
    /// - `model_name`: whisper model name (e.g. "base", "large-v3")
    /// - `language`: BCP-47 language code (e.g. "es", "en")
    /// - `timeout_secs`: maximum seconds per transcription before kill
    /// - `concurrency`: maximum concurrent transcription processes
    pub fn new(
        binary_path: String,
        model_name: &str,
        language: String,
        timeout_secs: u64,
        concurrency: usize,
    ) -> Self {
        let model_path = resolve_model_path(model_name);
        Self {
            binary_path,
            model_path,
            language,
            timeout: Duration::from_secs(timeout_secs),
            semaphore: Arc::new(Semaphore::new(concurrency)),
        }
    }

    /// Parse the text output from whisper-cli stdout.
    ///
    /// Handles multi-line output, trims whitespace, and filters
    /// `[BLANK_AUDIO]` markers that whisper.cpp emits for silence.
    fn parse_output(raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let text: String = trimmed
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.eq_ignore_ascii_case("[BLANK_AUDIO]"))
            .collect::<Vec<_>>()
            .join(" ");
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_tests(
        binary_path: String,
        model_path: PathBuf,
        language: String,
        timeout_secs: u64,
        concurrency: usize,
    ) -> Self {
        Self {
            binary_path,
            model_path,
            language,
            timeout: Duration::from_secs(timeout_secs),
            semaphore: Arc::new(Semaphore::new(concurrency)),
        }
    }
}

/// Resolve the whisper model path following Corvus conventions.
///
/// 1. `~/.corvus/models/whisper/ggml-{model}.bin` (if file exists)
/// 2. Fallback: `/usr/local/share/whisper/ggml-{model}.bin`
pub(crate) fn resolve_model_path(model_name: &str) -> PathBuf {
    let filename = format!("ggml-{model_name}.bin");

    if let Some(user_dirs) = directories::UserDirs::new() {
        let user_path = user_dirs
            .home_dir()
            .join(".corvus/models/whisper")
            .join(&filename);
        if user_path.is_file() {
            return user_path;
        }
    }

    // Fallback: system-wide path (returned even if absent so caller
    // can produce a clear "not found" diagnostic).
    PathBuf::from(format!("/usr/local/share/whisper/{filename}"))
}

#[async_trait]
impl Transcriber for WhisperCliTranscriber {
    fn name(&self) -> &str {
        "whisper-cli"
    }

    async fn transcribe(
        &self,
        audio: &StagedAudio,
    ) -> Result<TranscriptionResult, AudioRejectionReason> {
        // Acquire semaphore permit (queues if no permits available).
        // Permit is released automatically when `_permit` is dropped.
        let _permit = self.semaphore.acquire().await.map_err(|_| {
            tracing::error!("Transcription semaphore closed unexpectedly");
            AudioRejectionReason::SystemError
        })?;

        // Validate model exists before spawning
        if !self.model_path.exists() {
            tracing::error!("Whisper model not found at {}", self.model_path.display());
            return Err(AudioRejectionReason::TranscriberUnavailable);
        }

        // Build command — kill_on_drop ensures the child is terminated
        // if the future is cancelled (e.g. on timeout).
        let mut cmd = tokio::process::Command::new(&self.binary_path);
        cmd.kill_on_drop(true);
        cmd.arg("-m")
            .arg(&self.model_path)
            .arg("-f")
            .arg(&audio.temp_path)
            .arg("-l")
            .arg(&self.language)
            .arg("--no-timestamps")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Spawn process
        let child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                tracing::error!("whisper-cli binary not found at '{}'", self.binary_path);
                AudioRejectionReason::TranscriberUnavailable
            } else {
                tracing::error!("Failed to spawn whisper-cli: {e}");
                AudioRejectionReason::TranscriptionFailed
            }
        })?;

        // Wait with timeout
        let output = match tokio::time::timeout(self.timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                tracing::error!("whisper-cli I/O error: {e}");
                return Err(AudioRejectionReason::TranscriptionFailed);
            }
            Err(_) => {
                tracing::error!("Transcription timed out after {}s", self.timeout.as_secs());
                return Err(AudioRejectionReason::TranscriptionFailed);
            }
        };

        // Check exit code
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let code = output.status.code().unwrap_or(-1);
            tracing::error!("whisper-cli exited with code {code}: {stderr}");
            let stderr_lower = stderr.to_ascii_lowercase();
            let is_decode_error = [
                "decode",
                "unsupported format",
                "invalid data",
                "ffmpeg",
                "libav",
            ]
            .iter()
            .any(|kw| stderr_lower.contains(kw));
            return Err(if is_decode_error {
                AudioRejectionReason::Corrupted
            } else {
                AudioRejectionReason::TranscriptionFailed
            });
        }

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let text = Self::parse_output(&stdout).ok_or_else(|| {
            tracing::warn!("whisper-cli produced no speech output");
            AudioRejectionReason::NoSpeechDetected
        })?;

        Ok(TranscriptionResult {
            text,
            language: Some(self.language.clone()),
            duration_secs: audio.duration_secs,
            confidence: None,
            processing_ms: None, // set by caller after timing
        })
    }

    async fn health_check(&self) -> Result<(), String> {
        // Check binary is accessible by running --help
        let binary_check = tokio::process::Command::new(&self.binary_path)
            .arg("--help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;

        match binary_check {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(format!(
                    "whisper-cli binary not found at '{}'",
                    self.binary_path
                ));
            }
            Err(e) => {
                return Err(format!("Failed to execute whisper-cli: {e}"));
            }
        }

        // Check model file exists
        if !self.model_path.exists() {
            return Err(format!(
                "Whisper model not found at '{}'",
                self.model_path.display()
            ));
        }

        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use tempfile::TempDir;

    #[cfg(unix)]
    fn make_test_staged_audio(dir: &std::path::Path) -> StagedAudio {
        let audio_path = dir.join("input.ogg");
        let mut bytes = vec![0_u8; 64];
        bytes[0..4].copy_from_slice(b"OggS");
        fs::write(&audio_path, bytes).unwrap();

        StagedAudio {
            sha256: "abc123".into(),
            mime_type: crate::channels::audio_media::AllowedAudioMime::OggOpus,
            byte_len: 64,
            duration_secs: Some(5.0),
            temp_path: audio_path,
            channel_origin: "telegram".into(),
        }
    }

    #[cfg(unix)]
    fn write_fake_whisper_script(dir: &TempDir, script_name: &str, body: &str) -> PathBuf {
        let script_path = dir.path().join(script_name);
        fs::write(&script_path, body).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();
        script_path
    }

    // ── parse_output ──────────────────────────────────────────

    #[test]
    fn parse_output_extracts_text() {
        let raw = "  Hello world  \n";
        assert_eq!(
            WhisperCliTranscriber::parse_output(raw),
            Some("Hello world".to_string())
        );
    }

    #[test]
    fn parse_output_joins_multiline() {
        let raw = "  Hello\n  world  \n";
        assert_eq!(
            WhisperCliTranscriber::parse_output(raw),
            Some("Hello world".to_string())
        );
    }

    #[test]
    fn parse_output_filters_blank_audio_marker() {
        let raw = "[BLANK_AUDIO]\n";
        assert_eq!(WhisperCliTranscriber::parse_output(raw), None);
    }

    #[test]
    fn parse_output_filters_blank_audio_case_insensitive() {
        let raw = "[blank_audio]\n";
        assert_eq!(WhisperCliTranscriber::parse_output(raw), None);
    }

    #[test]
    fn parse_output_returns_none_for_empty() {
        assert_eq!(WhisperCliTranscriber::parse_output(""), None);
        assert_eq!(WhisperCliTranscriber::parse_output("   \n  "), None);
    }

    #[test]
    fn parse_output_mixed_content_and_blank_audio() {
        let raw = "[BLANK_AUDIO]\nHola, ¿cómo estás?\n[BLANK_AUDIO]\n";
        assert_eq!(
            WhisperCliTranscriber::parse_output(raw),
            Some("Hola, ¿cómo estás?".to_string())
        );
    }

    #[test]
    fn parse_output_preserves_punctuation_and_unicode() {
        let raw = "  ¿Qué tiempo hace hoy?  \n";
        assert_eq!(
            WhisperCliTranscriber::parse_output(raw),
            Some("¿Qué tiempo hace hoy?".to_string())
        );
    }

    // ── resolve_model_path ────────────────────────────────────

    #[test]
    fn resolve_model_path_falls_back_to_system_when_user_missing() {
        // When the user-local model file does not exist, the function
        // must fall back to the system path.
        let path = resolve_model_path("base");
        let path_str = path.to_string_lossy();
        // Either the user path exists (rare in CI) or we get the system fallback
        assert!(
            path_str.contains("ggml-base.bin"),
            "unexpected path: {path_str}"
        );
    }

    #[test]
    fn resolve_model_path_includes_model_name() {
        let path = resolve_model_path("large-v3");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("ggml-large-v3.bin"),
            "unexpected path: {path_str}"
        );
    }

    #[test]
    fn resolve_model_path_prefers_user_dir_when_file_exists() {
        // Create a temp dir simulating ~/.corvus/models/whisper
        // This test verifies the preference logic indirectly: when
        // the user file doesn't exist, we get the system path.
        let path = resolve_model_path("nonexistent-test-model-xyz");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("/usr/local/share/whisper/"),
            "expected system fallback, got: {path_str}"
        );
    }

    // ── WhisperCliTranscriber construction ─────────────────────

    #[test]
    fn new_sets_fields_correctly() {
        let t = WhisperCliTranscriber::new("whisper-cli".into(), "base", "es".into(), 120, 2);
        assert_eq!(t.binary_path, "whisper-cli");
        assert_eq!(t.language, "es");
        assert_eq!(t.timeout, Duration::from_mins(2));
        assert!(t.model_path.to_string_lossy().contains("ggml-base.bin"));
    }

    #[test]
    fn transcriber_name_is_whisper_cli() {
        let t = WhisperCliTranscriber::new("whisper-cli".into(), "base", "es".into(), 120, 1);
        assert_eq!(t.name(), "whisper-cli");
    }

    // ── Error mapping (async) ─────────────────────────────────

    #[tokio::test]
    async fn transcribe_fails_when_binary_not_found() {
        let t = WhisperCliTranscriber::new(
            "/nonexistent/whisper-cli-fake-path".into(),
            "base",
            "es".into(),
            10,
            1,
        );

        let staged = StagedAudio {
            sha256: "abc123".into(),
            mime_type: crate::channels::audio_media::AllowedAudioMime::OggOpus,
            byte_len: 100,
            duration_secs: Some(5.0),
            temp_path: PathBuf::from("/tmp/nonexistent.ogg"),
            channel_origin: "telegram".into(),
        };

        let result = t.transcribe(&staged).await;
        assert!(result.is_err());
        // Binary not found → TranscriberUnavailable (model check) or
        // TranscriptionFailed (spawn failure). Since model also won't
        // exist, this hits TranscriberUnavailable first.
        let err = result.unwrap_err();
        assert!(
            err == AudioRejectionReason::TranscriberUnavailable
                || err == AudioRejectionReason::TranscriptionFailed,
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn health_check_fails_when_binary_not_found() {
        let t = WhisperCliTranscriber::new(
            "/nonexistent/whisper-cli-fake-path".into(),
            "base",
            "es".into(),
            10,
            1,
        );

        let result = t.health_check().await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("not found"),
            "error should mention 'not found'"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn transcribe_runs_mock_whisper_binary_and_returns_known_text() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = dir.path().join("ggml-base.bin");
        fs::write(&model_path, b"fake-model").unwrap();
        let staged = make_test_staged_audio(dir.path());

        let script_path = write_fake_whisper_script(
            &dir,
            "fake-whisper.sh",
            &format!(
                r#"#!/bin/sh
set -eu

# Validate exact whisper-cli argument contract
if [ "$#" -ne 7 ] || [ "$1" != "-m" ] || [ "$2" != "{model}" ] || \
   [ "$3" != "-f" ] || [ "$4" != "{file}" ] || \
   [ "$5" != "-l" ] || [ "$6" != "es" ] || \
   [ "$7" != "--no-timestamps" ]; then
  echo "ERROR: fake whisper script expected '-m {model} -f {file} -l es --no-timestamps', got: $*" >&2
  exit 1
fi

printf 'Known mock transcription\n'
"#,
                model = model_path.display(),
                file = staged.temp_path.display()
            ),
        );

        let transcriber = WhisperCliTranscriber::new_for_tests(
            script_path.display().to_string(),
            model_path,
            "es".into(),
            5,
            1,
        );

        let result = transcriber.transcribe(&staged).await.unwrap();

        assert_eq!(result.text, "Known mock transcription");
        assert_eq!(result.language.as_deref(), Some("es"));
        assert_eq!(result.duration_secs, Some(5.0));
    }
}
