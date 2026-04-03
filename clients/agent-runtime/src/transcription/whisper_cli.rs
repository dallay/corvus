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
}

/// Resolve the whisper model path following Corvus conventions.
///
/// 1. `~/.corvus/models/whisper/ggml-{model}.bin`
/// 2. Fallback: `/usr/local/share/whisper/ggml-{model}.bin`
pub(crate) fn resolve_model_path(model_name: &str) -> PathBuf {
    let filename = format!("ggml-{model_name}.bin");

    if let Some(user_dirs) = directories::UserDirs::new() {
        return user_dirs
            .home_dir()
            .join(".corvus/models/whisper")
            .join(&filename);
    }

    // Fallback when home directory cannot be determined
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

        // Build command
        let mut cmd = tokio::process::Command::new(&self.binary_path);
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
            return Err(AudioRejectionReason::Corrupted);
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
    fn resolve_model_path_uses_corvus_dir() {
        let path = resolve_model_path("base");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains(".corvus/models/whisper/ggml-base.bin"),
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

    // ── WhisperCliTranscriber construction ─────────────────────

    #[test]
    fn new_sets_fields_correctly() {
        let t = WhisperCliTranscriber::new("whisper-cli".into(), "base", "es".into(), 120, 2);
        assert_eq!(t.binary_path, "whisper-cli");
        assert_eq!(t.language, "es");
        assert_eq!(t.timeout, Duration::from_secs(120));
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
}
