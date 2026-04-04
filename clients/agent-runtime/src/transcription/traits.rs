use async_trait::async_trait;

use crate::channels::audio_media::{AudioRejectionReason, StagedAudio};

/// Result of a successful audio transcription.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    /// The transcribed text.
    pub text: String,
    /// Detected or forced language code.
    pub language: Option<String>,
    /// Actual audio duration as reported by the transcription engine.
    pub duration_secs: Option<f64>,
    /// Engine-reported confidence (0.0–1.0), if available.
    pub confidence: Option<f64>,
    /// Wall-clock processing time in milliseconds.
    /// Set by the caller after `transcribe()` returns.
    pub processing_ms: Option<u64>,
}

/// Extension point for speech-to-text engines.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks. The Phase 1 implementation is `WhisperCliTranscriber`
/// which wraps whisper.cpp as an external process.
#[async_trait]
pub trait Transcriber: Send + Sync {
    /// Human-readable name of the transcription engine.
    fn name(&self) -> &str;

    /// Transcribe a staged audio file to text.
    ///
    /// Returns `AudioRejectionReason` on failure so the caller can
    /// emit the correct observability event and user message.
    async fn transcribe(
        &self,
        audio: &StagedAudio,
    ) -> Result<TranscriptionResult, AudioRejectionReason>;

    /// Whether the engine is ready (binary found, model available).
    ///
    /// Returns `Ok(())` if healthy, or `Err(reason)` describing the
    /// issue for doctor/startup diagnostics.
    async fn health_check(&self) -> Result<(), String>;
}
