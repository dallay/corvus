use std::sync::Arc;

use super::traits::{Channel, ChannelMessage, SendMessage};
use crate::channels::audio_media::{stage_audio_from_bytes, AudioRejectionReason, MAX_AUDIO_BYTES};
use crate::config::AudioConfig;
use crate::observability::{
    AudioIngressEvent, AudioIngressOutcome, AudioIngressReason, NoopObserver, Observer,
};
use crate::transcription::traits::Transcriber;
use async_trait::async_trait;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use uuid::Uuid;

// ── CliChannel ────────────────────────────────────────────────

/// CLI channel — stdin/stdout, always available, zero deps.
///
/// Create with [`CliChannel::new()`] for text-only mode (backward-compatible),
/// or [`CliChannel::with_audio()`] to enable the `/audio <path>` command.
pub struct CliChannel {
    /// Optional transcriber for `/audio` command support.
    /// `None` → graceful "not available" message when `/audio` is typed.
    transcriber: Option<Arc<dyn Transcriber>>,
    /// Audio configuration — gating and limits.
    audio_config: AudioConfig,
    /// Observer for `AudioIngressEvent` telemetry.
    observer: Arc<dyn Observer>,
}

impl CliChannel {
    /// Create a bare CLI channel with no audio support.
    ///
    /// Backward-compatible with `run_interactive()` which has no transcriber.
    /// Attempting `/audio` will print a user-friendly "not available" message.
    pub fn new() -> Self {
        Self {
            transcriber: None,
            audio_config: AudioConfig::default(),
            observer: Arc::new(NoopObserver),
        }
    }

    /// Create a CLI channel with full audio support.
    ///
    /// `transcriber: None` → audio pipeline gated at the transcriber check.
    pub fn with_audio(
        transcriber: Option<Arc<dyn Transcriber>>,
        audio_config: AudioConfig,
        observer: Arc<dyn Observer>,
    ) -> Self {
        Self {
            transcriber,
            audio_config,
            observer,
        }
    }

    /// Execute the `/audio <path>` pipeline (Option A — pre-pipeline).
    ///
    /// Gate checks → file metadata pre-check → read bytes → stage →
    /// transcribe → print feedback → emit `AudioIngressEvent` → send
    /// text `ChannelMessage` into the normal agent flow.
    ///
    /// `staged.cleanup()` is called on **all** exit paths after staging
    /// succeeds to ensure the temp file is always removed.
    async fn handle_audio_command(
        &self,
        path: &str,
        tx: &tokio::sync::mpsc::Sender<ChannelMessage>,
    ) {
        let started_at = std::time::Instant::now();

        // ── Gate 1: audio globally enabled ───────────────────
        if !self.audio_config.enabled {
            println!("Audio input is currently disabled.");
            self.emit_rejected(&AudioRejectionReason::Disabled, None, None);
            return;
        }

        // ── Gate 2: "cli" in allowed_channels ────────────────
        if !self
            .audio_config
            .allowed_channels
            .iter()
            .any(|c| c == "cli")
        {
            println!("Audio input is not enabled for CLI.");
            self.emit_rejected(&AudioRejectionReason::ChannelNotAllowed, None, None);
            return;
        }

        // ── Gate 3: transcriber available ────────────────────
        let transcriber = match &self.transcriber {
            Some(t) => Arc::clone(t),
            None => {
                println!(
                    "Audio transcription is not available on this agent. \
                     Please send text instead."
                );
                self.emit_rejected(&AudioRejectionReason::TranscriberUnavailable, None, None);
                return;
            }
        };

        // ── Pre-check: file metadata (existence + size) ──────
        let metadata = match tokio::fs::metadata(path).await {
            Ok(m) => m,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    println!("File not found: {path}");
                } else {
                    println!("Cannot read file: {path}");
                }
                // No AudioIngressEvent — the audio pipeline was never entered.
                return;
            }
        };

        let file_size = metadata.len();
        if file_size > self.audio_config.max_audio_bytes {
            let reason = AudioRejectionReason::Oversize;
            println!(
                "{}",
                cli_rejection_message(
                    &reason,
                    Some(file_size),
                    Some(self.audio_config.max_audio_bytes)
                )
            );
            self.emit_rejected(&reason, Some(file_size), None);
            return;
        }

        // ── Read bytes ────────────────────────────────────────
        let bytes = match tokio::fs::read(path).await {
            Ok(b) => b,
            Err(_) => {
                println!("Cannot read file: {path}");
                return;
            }
        };

        // ── Stage audio (MIME sniff, SHA-256, temp file) ──────
        let staged = match stage_audio_from_bytes(
            &bytes,
            "cli",
            None,
            None,
            self.audio_config.max_audio_bytes,
            self.audio_config.max_audio_duration_secs,
            "cli",
        )
        .await
        {
            Ok(s) => s,
            Err(reason) => {
                println!(
                    "{}",
                    cli_rejection_message(
                        &reason,
                        Some(bytes.len() as u64),
                        Some(self.audio_config.max_audio_bytes)
                    )
                );
                self.emit_rejected(&reason, Some(bytes.len() as u64), None);
                return;
            }
        };

        // Save needed fields before cleanup consumes the temp file reference.
        let staged_byte_len = staged.byte_len;
        let staged_mime = staged.mime_type.as_str().to_string();
        let staged_duration = staged.duration_secs;

        // ── Transcribe — staged.cleanup() on ALL exit paths ──
        let transcription_result = match transcriber.transcribe(&staged).await {
            Ok(r) => r,
            Err(reason) => {
                staged.cleanup();
                println!(
                    "{}",
                    cli_rejection_message(
                        &reason,
                        Some(staged_byte_len),
                        Some(self.audio_config.max_audio_bytes)
                    )
                );
                self.emit_rejected(&reason, Some(staged_byte_len), staged_duration);
                return;
            }
        };
        staged.cleanup(); // success path

        let elapsed_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);

        // ── Empty transcription guard (REQ-14) ────────────────
        let text = transcription_result.text.trim().to_string();
        if text.is_empty() {
            println!("No speech detected in the audio file.");
            self.emit_rejected(
                &AudioRejectionReason::NoSpeechDetected,
                Some(staged_byte_len),
                transcription_result.duration_secs.or(staged_duration),
            );
            return;
        }

        // ── Print transcription feedback ──────────────────────
        println!("[Transcription]: \"{text}\"");

        // ── Emit admitted AudioIngressEvent ───────────────────
        self.observer.on_audio_ingress(&AudioIngressEvent {
            channel: "cli".to_string(),
            outcome: AudioIngressOutcome::Admitted,
            reason: None,
            mime_type: Some(staged_mime),
            byte_len: Some(staged_byte_len),
            duration_secs: transcription_result.duration_secs.or(staged_duration),
            transcription_duration_ms: Some(elapsed_ms),
        });

        // ── Send as normal text ChannelMessage ────────────────
        let msg = ChannelMessage {
            id: Uuid::new_v4().to_string(),
            sender: "user".to_string(),
            reply_target: "user".to_string(),
            content: text,
            channel: "cli".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            parts: vec![],
        };
        // Match the error handling in the normal text path: break if receiver is gone.
        if tx.send(msg).await.is_err() {
            tracing::debug!("CLI receiver channel closed, stopping listen loop");
        }
    }

    /// Emit a rejected `AudioIngressEvent` through the observer.
    fn emit_rejected(
        &self,
        reason: &AudioRejectionReason,
        byte_len: Option<u64>,
        duration_secs: Option<f64>,
    ) {
        self.observer.on_audio_ingress(&AudioIngressEvent {
            channel: "cli".to_string(),
            outcome: AudioIngressOutcome::Rejected,
            reason: Some(audio_rejection_to_ingress_reason(reason)),
            mime_type: None,
            byte_len,
            duration_secs,
            transcription_duration_ms: None,
        });
    }
}

#[async_trait]
impl Channel for CliChannel {
    fn name(&self) -> &str {
        "cli"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        println!("{}", message.content);
        Ok(())
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            if line == "/quit" || line == "/exit" {
                break;
            }

            // ── /audio command ────────────────────────────────
            if line.starts_with("/audio") {
                match parse_audio_command(&line) {
                    Some(Ok(path)) => {
                        self.handle_audio_command(&path, &tx).await;
                    }
                    Some(Err(())) => {
                        println!("Usage: /audio <file-path>");
                    }
                    None => {
                        // Starts with "/audio" but parse_audio_command rejected it
                        // (e.g. "/audiobook") — fall through to normal text path.
                        let msg = ChannelMessage {
                            id: Uuid::new_v4().to_string(),
                            sender: "user".to_string(),
                            reply_target: "user".to_string(),
                            content: line,
                            channel: "cli".to_string(),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            parts: vec![],
                        };
                        if tx.send(msg).await.is_err() {
                            break;
                        }
                    }
                }
                continue;
            }

            // ── Normal text message (unchanged path) ──────────
            let msg = ChannelMessage {
                id: Uuid::new_v4().to_string(),
                sender: "user".to_string(),
                reply_target: "user".to_string(),
                content: line,
                channel: "cli".to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                parts: vec![],
            };

            if tx.send(msg).await.is_err() {
                break;
            }
        }
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────

/// Parse a `/audio <path>` CLI input.
///
/// Returns:
/// - `None` — input is not an `/audio` command (e.g. `/audiobook`)
/// - `Some(Ok(path))` — valid `/audio <path>` with leading `~` expanded
/// - `Some(Err(()))` — bare `/audio` with no path argument (show usage)
pub(crate) fn parse_audio_command(input: &str) -> Option<Result<String, ()>> {
    if !input.starts_with("/audio") {
        return None;
    }
    let after = &input["/audio".len()..];
    // Ensure the character after "/audio" is whitespace or end-of-input,
    // to distinguish "/audio /path" from "/audiobook".
    match after.chars().next() {
        None => return Some(Err(())),                 // exactly "/audio"
        Some(c) if !c.is_whitespace() => return None, // "/audioXXX" — not this command
        _ => {}
    }
    let rest = after.trim();
    if rest.is_empty() {
        return Some(Err(())); // "/audio   " — only whitespace
    }
    Some(Ok(expand_home_tilde(rest)))
}

/// Expand a leading `~` to the user's home directory.
///
/// If `home` is provided, use it directly. Otherwise, query `$HOME` from the environment.
/// This overload allows tests to inject a synthetic HOME value without mutating the global environment.
pub(crate) fn expand_home_tilde_with_home(path: &str, home: Option<&str>) -> String {
    let home = if let Some(h) = home {
        h.to_string()
    } else {
        std::env::var("HOME").unwrap_or_else(|_| "~".to_string())
    };
    if path == "~" {
        return home;
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return format!("{home}/{rest}");
    }
    path.to_string()
}

/// Expand a leading `~` to the user's home directory (`$HOME`).
///
/// - `~` alone → `$HOME`
/// - `~/path` → `$HOME/path`
/// - All other paths returned unchanged.
pub(crate) fn expand_home_tilde(path: &str) -> String {
    expand_home_tilde_with_home(path, None)
}

/// Map an `AudioRejectionReason` to the corresponding `AudioIngressReason`.
fn audio_rejection_to_ingress_reason(reason: &AudioRejectionReason) -> AudioIngressReason {
    match reason {
        AudioRejectionReason::Disabled => AudioIngressReason::Disabled,
        AudioRejectionReason::ChannelNotAllowed => AudioIngressReason::ChannelNotAllowed,
        AudioRejectionReason::FetchFailed => AudioIngressReason::FetchFailed,
        AudioRejectionReason::MimeRejected => AudioIngressReason::MimeRejected,
        AudioRejectionReason::Oversize => AudioIngressReason::Oversize,
        AudioRejectionReason::TooLong => AudioIngressReason::TooLong,
        AudioRejectionReason::Corrupted => AudioIngressReason::Corrupted,
        AudioRejectionReason::TranscriptionFailed => AudioIngressReason::TranscriptionFailed,
        AudioRejectionReason::NoSpeechDetected => AudioIngressReason::NoSpeechDetected,
        AudioRejectionReason::TranscriberUnavailable => AudioIngressReason::TranscriberUnavailable,
        AudioRejectionReason::MultipleAudioParts => AudioIngressReason::MultipleAudioParts,
        AudioRejectionReason::SystemError => AudioIngressReason::SystemError,
    }
}

/// User-facing error message for a CLI audio rejection reason.
/// If `actual_value` is Some, include it in the message (for Oversize/TooLong).
/// If `max_value` is Some, include the configured limit.
fn cli_rejection_message(
    reason: &AudioRejectionReason,
    actual_value: Option<u64>,
    max_value: Option<u64>,
) -> String {
    match reason {
        AudioRejectionReason::Disabled => "Audio input is currently disabled.".to_string(),
        AudioRejectionReason::ChannelNotAllowed => {
            "Audio input is not enabled for CLI.".to_string()
        }
        AudioRejectionReason::MimeRejected => {
            "That audio format is not supported. Supported formats: OGG, MP3, WAV, M4A.".to_string()
        }
        AudioRejectionReason::Oversize => {
            let max = max_value.unwrap_or(MAX_AUDIO_BYTES);
            if let Some(actual) = actual_value {
                format!("Audio file is too large ({actual} bytes). Maximum size is {max} bytes.",)
            } else {
                format!("Audio file is too large. Maximum size is {max} bytes.")
            }
        }
        AudioRejectionReason::TooLong => {
            let max = max_value.unwrap_or(600);
            if let Some(actual) = actual_value {
                format!(
                    "Audio file is too long ({} seconds). Maximum duration is {max} seconds.",
                    actual
                )
            } else {
                format!("Audio file is too long. Maximum duration is {max} seconds.")
            }
        }
        AudioRejectionReason::TranscriberUnavailable => {
            "Audio transcription is not available right now. Please send text instead.".to_string()
        }
        AudioRejectionReason::TranscriptionFailed => {
            "Transcription failed. Please try again or send text instead.".to_string()
        }
        AudioRejectionReason::NoSpeechDetected => {
            "No speech detected in the audio file.".to_string()
        }
        AudioRejectionReason::FetchFailed | AudioRejectionReason::Corrupted => {
            "Cannot process the audio file. It may be corrupted.".to_string()
        }
        AudioRejectionReason::MultipleAudioParts => {
            "Multiple audio parts are not supported in CLI mode.".to_string()
        }
        AudioRejectionReason::SystemError => {
            "A system error occurred during audio processing.".to_string()
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::audio_media::StagedAudio;
    use crate::transcription::traits::{Transcriber, TranscriptionResult};
    use std::sync::Mutex;

    // ── Mock transcriber for pipeline tests ───────────────────

    struct OkTranscriber;

    #[async_trait]
    impl Transcriber for OkTranscriber {
        fn name(&self) -> &str {
            "mock-ok"
        }

        async fn transcribe(
            &self,
            _audio: &StagedAudio,
        ) -> Result<TranscriptionResult, AudioRejectionReason> {
            Ok(TranscriptionResult {
                text: "hello world".to_string(),
                language: Some("en".to_string()),
                duration_secs: Some(3.0),
                confidence: None,
                processing_ms: None,
            })
        }

        async fn health_check(&self) -> Result<(), String> {
            Ok(())
        }
    }

    // ── Test observer that records audio ingress events ───────

    #[derive(Debug, Default)]
    struct TestObserver {
        audio_events: Mutex<Vec<AudioIngressEvent>>,
    }

    impl TestObserver {
        fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        fn events(&self) -> Vec<AudioIngressEvent> {
            self.audio_events.lock().unwrap().clone()
        }
    }

    impl Observer for TestObserver {
        fn name(&self) -> &str {
            "test"
        }

        fn record_event(&self, _event: &crate::observability::ObserverEvent) {}

        fn record_metric(&self, _metric: &crate::observability::ObserverMetric) {}

        fn on_audio_ingress(&self, event: &AudioIngressEvent) {
            self.audio_events.lock().unwrap().push(event.clone());
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    // ── T3.4: parse_audio_command tests ──────────────────────

    #[test]
    fn test_cli_audio_no_path() {
        // Bare "/audio" with no path → usage hint (Err)
        assert_eq!(parse_audio_command("/audio"), Some(Err(())));
    }

    #[test]
    fn test_cli_audio_no_path_trailing_spaces() {
        // "/audio   " with only spaces → also usage hint
        assert_eq!(parse_audio_command("/audio   "), Some(Err(())));
    }

    #[test]
    fn test_cli_audio_absolute_path() {
        let result = parse_audio_command("/audio /tmp/test.ogg");
        assert_eq!(result, Some(Ok("/tmp/test.ogg".to_string())));
    }

    #[test]
    fn test_cli_audio_relative_path() {
        let result = parse_audio_command("/audio recording.mp3");
        assert_eq!(result, Some(Ok("recording.mp3".to_string())));
    }

    #[test]
    fn test_cli_audio_relative_path_with_parent() {
        let result = parse_audio_command("/audio ../recordings/note.ogg");
        assert_eq!(result, Some(Ok("../recordings/note.ogg".to_string())));
    }

    #[test]
    fn test_cli_audio_tilde_expansion() {
        // Use the home-aware helper to avoid global env mutation.
        let result = expand_home_tilde_with_home("~/test.ogg", Some("/home/testuser"));
        assert_eq!(result, "/home/testuser/test.ogg".to_string());
    }

    #[test]
    fn test_cli_audio_tilde_alone() {
        let result = expand_home_tilde_with_home("~", Some("/home/testuser"));
        assert_eq!(result, "/home/testuser".to_string());
    }

    #[test]
    fn test_cli_audio_not_audio_command() {
        // A different command is not parsed as /audio
        assert_eq!(parse_audio_command("/quit"), None);
        assert_eq!(parse_audio_command("hello"), None);
        assert_eq!(parse_audio_command(""), None);
    }

    #[test]
    fn test_cli_audio_command_prefix_not_matched() {
        // "/audiobook" must not be parsed as an audio command
        assert_eq!(parse_audio_command("/audiobook /path"), None);
        assert_eq!(parse_audio_command("/audiofile"), None);
    }

    // ── T3.4: error condition tests ───────────────────────────

    #[tokio::test]
    async fn test_cli_audio_disabled() {
        // Audio globally disabled → "Audio input is currently disabled." printed,
        // no ChannelMessage sent on tx, Rejected event emitted.
        let observer = TestObserver::new();
        let ch = CliChannel::with_audio(
            Some(Arc::new(OkTranscriber)),
            AudioConfig {
                enabled: false,
                allowed_channels: vec!["cli".to_string()],
                ..AudioConfig::default()
            },
            Arc::clone(&observer) as Arc<dyn Observer>,
        );

        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        ch.handle_audio_command("/tmp/fake.ogg", &tx).await;

        // No message should be sent on tx
        assert!(
            rx.try_recv().is_err(),
            "no ChannelMessage should be sent when disabled"
        );

        // Observer should have a Rejected event with Disabled reason
        let events = observer.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].outcome, AudioIngressOutcome::Rejected));
        assert!(matches!(
            events[0].reason,
            Some(AudioIngressReason::Disabled)
        ));
    }

    #[tokio::test]
    async fn test_cli_audio_not_in_allowed_channels() {
        // "cli" not in allowed_channels → rejected, no message sent.
        let observer = TestObserver::new();
        let ch = CliChannel::with_audio(
            Some(Arc::new(OkTranscriber)),
            AudioConfig {
                enabled: true,
                allowed_channels: vec!["telegram".to_string()], // no "cli"
                ..AudioConfig::default()
            },
            Arc::clone(&observer) as Arc<dyn Observer>,
        );

        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        ch.handle_audio_command("/tmp/fake.ogg", &tx).await;

        assert!(
            rx.try_recv().is_err(),
            "no ChannelMessage should be sent when channel not allowed"
        );

        let events = observer.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].outcome, AudioIngressOutcome::Rejected));
        assert!(matches!(
            events[0].reason,
            Some(AudioIngressReason::ChannelNotAllowed)
        ));
    }

    #[tokio::test]
    async fn test_cli_audio_file_not_found() {
        // Non-existent file → "File not found" path, no ChannelMessage sent,
        // no AudioIngressEvent (file never entered the pipeline).
        let observer = TestObserver::new();
        let ch = CliChannel::with_audio(
            Some(Arc::new(OkTranscriber)),
            AudioConfig {
                enabled: true,
                allowed_channels: vec!["cli".to_string()],
                ..AudioConfig::default()
            },
            Arc::clone(&observer) as Arc<dyn Observer>,
        );

        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        ch.handle_audio_command("/nonexistent_corvus_test_path_xyz_987/audio.ogg", &tx)
            .await;

        // No ChannelMessage and no AudioIngressEvent (pre-pipeline failure)
        assert!(rx.try_recv().is_err(), "no ChannelMessage for missing file");
        assert!(
            observer.events().is_empty(),
            "no AudioIngressEvent for file-not-found"
        );
    }

    #[tokio::test]
    async fn test_cli_audio_transcriber_unavailable() {
        // Transcriber is None → "not available" message, no ChannelMessage.
        let observer = TestObserver::new();
        let ch = CliChannel::with_audio(
            None, // no transcriber
            AudioConfig {
                enabled: true,
                allowed_channels: vec!["cli".to_string()],
                ..AudioConfig::default()
            },
            Arc::clone(&observer) as Arc<dyn Observer>,
        );

        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        ch.handle_audio_command("/tmp/fake.ogg", &tx).await;

        assert!(rx.try_recv().is_err());
        let events = observer.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].reason,
            Some(AudioIngressReason::TranscriberUnavailable)
        ));
    }

    // ── Existing tests — unchanged behaviour ──────────────────

    #[test]
    fn cli_channel_name() {
        assert_eq!(CliChannel::new().name(), "cli");
    }

    #[tokio::test]
    async fn cli_channel_send_does_not_panic() {
        let ch = CliChannel::new();
        let result = ch
            .send(&SendMessage {
                content: "hello".into(),
                recipient: "user".into(),
                subject: None,
            })
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn cli_channel_send_empty_message() {
        let ch = CliChannel::new();
        let result = ch
            .send(&SendMessage {
                content: String::new(),
                recipient: String::new(),
                subject: None,
            })
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn cli_channel_health_check() {
        let ch = CliChannel::new();
        assert!(ch.health_check().await);
    }

    #[test]
    fn channel_message_struct() {
        let msg = ChannelMessage {
            id: "test-id".into(),
            sender: "user".into(),
            reply_target: "user".into(),
            content: "hello".into(),
            channel: "cli".into(),
            timestamp: 1_234_567_890,
            parts: vec![],
        };
        assert_eq!(msg.id, "test-id");
        assert_eq!(msg.sender, "user");
        assert_eq!(msg.reply_target, "user");
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.channel, "cli");
        assert_eq!(msg.timestamp, 1_234_567_890);
    }

    #[test]
    fn channel_message_clone() {
        let msg = ChannelMessage {
            id: "id".into(),
            sender: "s".into(),
            reply_target: "s".into(),
            content: "c".into(),
            channel: "ch".into(),
            timestamp: 0,
            parts: vec![],
        };
        let cloned = msg.clone();
        assert_eq!(cloned.id, msg.id);
        assert_eq!(cloned.content, msg.content);
    }
}
