use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Allowed image MIME types for ingress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowedImageMime {
    Jpeg,
    Png,
    Webp,
}

impl AllowedImageMime {
    /// Parse from a MIME string (e.g. `"image/jpeg"`).
    pub fn from_mime_str(s: &str) -> Option<Self> {
        match s {
            "image/jpeg" => Some(Self::Jpeg),
            "image/png" => Some(Self::Png),
            "image/webp" => Some(Self::Webp),
            _ => None,
        }
    }

    /// Return the canonical MIME string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
        }
    }
}

/// Transport encoding for the image payload sent to the provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageTransportForm {
    /// Raw bytes inlined in the provider request (MVP).
    InlineBytes,
}

/// A validated, staged image ready for provider dispatch.
#[derive(Debug, Clone)]
pub struct StagedImage {
    pub sha256: String,
    pub mime_type: AllowedImageMime,
    pub byte_len: u64,
    pub temp_path: PathBuf,
    pub transport_form: ImageTransportForm,
    pub channel_origin: String,
}

/// Compact metadata for an image that appeared in a prior conversation turn.
/// Stored in history instead of raw bytes to bound memory usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageHistoryMeta {
    /// MIME type string (e.g. "image/jpeg").
    pub mime: String,
    /// SHA-256 hex digest of the original image bytes.
    pub sha256: String,
    /// Original image size in bytes.
    pub byte_len: u64,
    /// Channel that originated the image.
    pub channel_origin: String,
    /// User-provided caption, if any.
    pub caption: Option<String>,
    /// Model-generated description of image content (populated post-response).
    pub description: Option<String>,
}

impl ImageHistoryMeta {
    /// Build from a `StagedImage` at ingestion time (description populated later).
    pub fn from_staged(staged: &StagedImage, caption: Option<String>) -> Self {
        Self {
            mime: staged.mime_type.as_str().to_string(),
            sha256: staged.sha256.clone(),
            byte_len: staged.byte_len,
            channel_origin: staged.channel_origin.clone(),
            caption,
            description: None,
        }
    }

    /// Render as a synthetic context string for history injection.
    pub fn to_context_string(&self) -> String {
        let prefix_len = 16.min(self.sha256.len());
        let mut s = format!(
            "[Prior image: {}, {} bytes, sha256:{}",
            self.mime,
            self.byte_len,
            &self.sha256[..prefix_len]
        );
        if let Some(desc) = &self.description {
            use std::fmt::Write;
            let sanitized = sanitize_history_text(desc);
            if !sanitized.is_empty() {
                let _ = write!(s, ". Description: {sanitized}");
            }
        }
        if let Some(cap) = &self.caption {
            use std::fmt::Write;
            let sanitized = sanitize_history_text(cap);
            if !sanitized.is_empty() {
                let _ = write!(s, ". Caption: {sanitized}");
            }
        }
        s.push(']');
        s
    }
}

/// Compact metadata for an audio turn stored in conversation history.
///
/// Stored in history instead of raw audio bytes to bound memory usage.
/// The transcription is stored at ingestion time (unlike images where
/// the description is populated post-response).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioHistoryMeta {
    /// MIME type string (e.g. "audio/ogg").
    pub mime: String,
    /// SHA-256 hex digest of the original audio bytes.
    pub sha256: String,
    /// Original audio size in bytes.
    pub byte_len: u64,
    /// Audio duration in seconds, if known.
    pub duration_secs: Option<f64>,
    /// Channel that originated the audio.
    pub channel_origin: String,
    /// The transcribed text from this audio.
    pub transcription: String,
    /// User-provided caption, if any.
    pub caption: Option<String>,
}

impl AudioHistoryMeta {
    /// Build directly from shared contract fields.
    pub fn new(
        mime: impl Into<String>,
        sha256: impl Into<String>,
        byte_len: u64,
        duration_secs: Option<f64>,
        channel_origin: impl Into<String>,
        transcription: impl Into<String>,
        caption: Option<String>,
    ) -> Self {
        Self {
            mime: mime.into(),
            sha256: sha256.into(),
            byte_len,
            duration_secs,
            channel_origin: channel_origin.into(),
            transcription: transcription.into(),
            caption,
        }
    }

    /// Render as a synthetic context string for history injection.
    ///
    /// Only includes modality, duration, transcription, and caption.
    /// Internal metadata (sha256, byte_len) is kept in the struct but not
    /// injected into model-facing history to reduce token consumption.
    ///
    /// Example: `"[Prior audio: audio/ogg, 45s. Transcription: Hola...]"`
    pub fn to_context_string(&self) -> String {
        let mut s = format!("[Prior audio: {}", self.mime);
        if let Some(dur) = self.duration_secs {
            use std::fmt::Write;
            let _ = write!(s, ", {dur:.0}s");
        }
        let sanitized = sanitize_history_text(&self.transcription);
        if !sanitized.is_empty() {
            use std::fmt::Write;
            let _ = write!(s, ". Transcription: {sanitized}");
        }
        if let Some(cap) = &self.caption {
            use std::fmt::Write;
            let sanitized_cap = sanitize_history_text(cap);
            if !sanitized_cap.is_empty() {
                let _ = write!(s, ". Caption: {sanitized_cap}");
            }
        }
        s.push(']');
        s
    }
}

fn sanitize_history_text(value: &str) -> String {
    value
        .chars()
        .filter(|c| *c != '\n' && *c != '\r')
        .take(200)
        .collect()
}
