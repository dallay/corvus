use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Maximum audio payload size (25 MiB).
pub const MAX_AUDIO_BYTES: u64 = 25 * 1024 * 1024;

/// Hard ceiling for `max_audio_bytes` config override (100 MiB).
pub const MAX_AUDIO_BYTES_CEILING: u64 = 100 * 1024 * 1024;

/// Maximum audio duration in seconds (10 minutes).
pub const MAX_AUDIO_DURATION_SECS: u64 = 600;

/// Hard ceiling for `max_audio_duration_secs` config override (1 hour).
pub const MAX_AUDIO_DURATION_SECS_CEILING: u64 = 3600;

// ── AllowedAudioMime ──────────────────────────────────────────

/// Allowed audio MIME types for ingress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowedAudioMime {
    /// OGG/Opus — Telegram voice notes.
    OggOpus,
    /// MPEG audio (MP3).
    Mp3,
    /// RIFF WAVE audio.
    Wav,
    /// MPEG-4 audio (M4A/AAC).
    M4a,
}

impl AllowedAudioMime {
    /// Parse from a MIME string (e.g. `"audio/ogg"`).
    pub fn from_mime_str(s: &str) -> Option<Self> {
        match s {
            "audio/ogg" | "audio/opus" | "audio/ogg; codecs=opus" => Some(Self::OggOpus),
            "audio/mpeg" | "audio/mp3" => Some(Self::Mp3),
            "audio/wav" | "audio/wave" | "audio/x-wav" => Some(Self::Wav),
            "audio/mp4" | "audio/m4a" | "audio/x-m4a" | "audio/aac" => Some(Self::M4a),
            _ => None,
        }
    }

    /// Return the canonical MIME string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::OggOpus => "audio/ogg",
            Self::Mp3 => "audio/mpeg",
            Self::Wav => "audio/wav",
            Self::M4a => "audio/mp4",
        }
    }

    /// Return the standard file extension (without leading dot).
    pub fn file_extension(&self) -> &str {
        match self {
            Self::OggOpus => "ogg",
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::M4a => "m4a",
        }
    }
}

// ── AudioRejectionReason ──────────────────────────────────────

/// Reason an audio turn was rejected.
///
/// Display strings are machine-readable identifiers matching the
/// `ImageRejectionReason` convention. User-facing messages are
/// constructed at the pipeline call-site (Phase 3).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AudioRejectionReason {
    #[error("disabled")]
    Disabled,
    #[error("channel_not_allowed")]
    ChannelNotAllowed,
    #[error("fetch_failed")]
    FetchFailed,
    #[error("mime_rejected")]
    MimeRejected,
    #[error("oversize")]
    Oversize,
    #[error("too_long")]
    TooLong,
    #[error("corrupted")]
    Corrupted,
    #[error("transcriber_unavailable")]
    TranscriberUnavailable,
    #[error("transcription_failed")]
    TranscriptionFailed,
    #[error("no_speech_detected")]
    NoSpeechDetected,
    #[error("multiple_audio_parts")]
    MultipleAudioParts,
    #[error("system_error")]
    SystemError,
}

// ── Magic-byte MIME sniffing ──────────────────────────────────

/// Validate audio MIME type by sniffing magic bytes first.
///
/// Magic-byte sniffing takes strict precedence over any declared MIME
/// from the channel (same security policy as `media::validate_mime`).
pub fn validate_audio_mime(
    declared: Option<&str>,
    sniffed_bytes: &[u8],
) -> Result<AllowedAudioMime, AudioRejectionReason> {
    // OGG: bytes 0-3 = "OggS" (0x4F 0x67 0x67 0x53)
    if sniffed_bytes.len() >= 4 && &sniffed_bytes[0..4] == b"OggS" {
        return Ok(AllowedAudioMime::OggOpus);
    }

    // MP3: ID3 tag header (0x49 0x44 0x33)
    if sniffed_bytes.len() >= 3 && &sniffed_bytes[0..3] == b"ID3" {
        return Ok(AllowedAudioMime::Mp3);
    }

    // MP3: MPEG sync word — first byte 0xFF, second byte has top 3 bits
    // set (0xE0 mask), non-zero layer bits (bits 1-2 != 0b00 = reserved),
    // and non-reserved version bits (bits 3-4 != 0b01 = reserved).
    // This excludes ADTS AAC headers and reserved MPEG frames.
    if sniffed_bytes.len() >= 2 && sniffed_bytes[0] == 0xFF {
        let b = sniffed_bytes[1];
        let sync_ok = (b & 0xE0) == 0xE0;
        let layer_ok = (b & 0x06) != 0; // layer bits != reserved (0b00)
        let version_ok = ((b >> 3) & 0x03) != 0x01; // version bits != reserved (0b01)
        if sync_ok && layer_ok && version_ok {
            return Ok(AllowedAudioMime::Mp3);
        }
    }

    // WAV: bytes 0-3 = "RIFF", bytes 8-11 = "WAVE"
    if sniffed_bytes.len() >= 12
        && &sniffed_bytes[0..4] == b"RIFF"
        && &sniffed_bytes[8..12] == b"WAVE"
    {
        return Ok(AllowedAudioMime::Wav);
    }

    // M4A: bytes 4-7 = "ftyp" (ISO base media file format)
    if sniffed_bytes.len() >= 8 && &sniffed_bytes[4..8] == b"ftyp" {
        return Ok(AllowedAudioMime::M4a);
    }

    // Magic bytes didn't match any known audio type.
    // Sniffing takes precedence — declared MIME is ignored for security.
    let _ = declared;
    Err(AudioRejectionReason::MimeRejected)
}

/// Validate that the audio size is within the allowed limit.
pub fn validate_audio_size(byte_len: u64, max_bytes: u64) -> Result<(), AudioRejectionReason> {
    if byte_len > max_bytes {
        Err(AudioRejectionReason::Oversize)
    } else {
        Ok(())
    }
}

/// Validate that the audio duration is within the allowed limit.
pub fn validate_audio_duration(
    duration_secs: u64,
    max_duration_secs: u64,
) -> Result<(), AudioRejectionReason> {
    if duration_secs > max_duration_secs {
        Err(AudioRejectionReason::TooLong)
    } else {
        Ok(())
    }
}

// ── StagedAudio ───────────────────────────────────────────────

/// A validated, staged audio file ready for transcription.
#[derive(Debug, Clone)]
pub struct StagedAudio {
    /// SHA-256 hex digest of the raw audio bytes.
    pub sha256: String,
    /// Validated MIME type from magic-byte sniffing.
    pub mime_type: AllowedAudioMime,
    /// Total byte size of the staged file.
    pub byte_len: u64,
    /// Duration if known (channel-declared or post-transcription).
    pub duration_secs: Option<f64>,
    /// Path to the temp file on disk.
    pub temp_path: PathBuf,
    /// Channel name that sourced the audio.
    pub channel_origin: String,
}

impl StagedAudio {
    /// Best-effort cleanup of the staged temp file.
    pub fn cleanup(&self) {
        if self.temp_path.exists() {
            if let Err(e) = std::fs::remove_file(&self.temp_path) {
                tracing::warn!(
                    "Failed to remove staged audio {}: {e}",
                    self.temp_path.display()
                );
            }
        }
    }
}

// ── AudioHistoryMeta ──────────────────────────────────────────

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
    /// Build from a `StagedAudio` after transcription completes.
    pub fn from_staged(staged: &StagedAudio, transcription: &str, caption: Option<&str>) -> Self {
        Self {
            mime: staged.mime_type.as_str().to_string(),
            sha256: staged.sha256.clone(),
            byte_len: staged.byte_len,
            duration_secs: staged.duration_secs,
            channel_origin: staged.channel_origin.clone(),
            transcription: transcription.to_string(),
            caption: caption.map(String::from),
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
        // Truncate transcription to 200 chars for history compactness
        let sanitized: String = self
            .transcription
            .chars()
            .filter(|c| *c != '\n' && *c != '\r')
            .take(200)
            .collect();
        if !sanitized.is_empty() {
            use std::fmt::Write;
            let _ = write!(s, ". Transcription: {sanitized}");
        }
        if let Some(cap) = &self.caption {
            use std::fmt::Write;
            let sanitized_cap: String = cap
                .chars()
                .filter(|c| *c != '\n' && *c != '\r')
                .take(200)
                .collect();
            if !sanitized_cap.is_empty() {
                let _ = write!(s, ". Caption: {sanitized_cap}");
            }
        }
        s.push(']');
        s
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── AllowedAudioMime round-trip (Task 2.1) ────────────────

    #[test]
    fn from_mime_str_ogg_variants() {
        assert_eq!(
            AllowedAudioMime::from_mime_str("audio/ogg"),
            Some(AllowedAudioMime::OggOpus)
        );
        assert_eq!(
            AllowedAudioMime::from_mime_str("audio/opus"),
            Some(AllowedAudioMime::OggOpus)
        );
        assert_eq!(
            AllowedAudioMime::from_mime_str("audio/ogg; codecs=opus"),
            Some(AllowedAudioMime::OggOpus)
        );
    }

    #[test]
    fn from_mime_str_mp3_variants() {
        assert_eq!(
            AllowedAudioMime::from_mime_str("audio/mpeg"),
            Some(AllowedAudioMime::Mp3)
        );
        assert_eq!(
            AllowedAudioMime::from_mime_str("audio/mp3"),
            Some(AllowedAudioMime::Mp3)
        );
    }

    #[test]
    fn from_mime_str_wav_variants() {
        assert_eq!(
            AllowedAudioMime::from_mime_str("audio/wav"),
            Some(AllowedAudioMime::Wav)
        );
        assert_eq!(
            AllowedAudioMime::from_mime_str("audio/wave"),
            Some(AllowedAudioMime::Wav)
        );
        assert_eq!(
            AllowedAudioMime::from_mime_str("audio/x-wav"),
            Some(AllowedAudioMime::Wav)
        );
    }

    #[test]
    fn from_mime_str_m4a_variants() {
        assert_eq!(
            AllowedAudioMime::from_mime_str("audio/mp4"),
            Some(AllowedAudioMime::M4a)
        );
        assert_eq!(
            AllowedAudioMime::from_mime_str("audio/m4a"),
            Some(AllowedAudioMime::M4a)
        );
        assert_eq!(
            AllowedAudioMime::from_mime_str("audio/x-m4a"),
            Some(AllowedAudioMime::M4a)
        );
        assert_eq!(
            AllowedAudioMime::from_mime_str("audio/aac"),
            Some(AllowedAudioMime::M4a)
        );
    }

    #[test]
    fn from_mime_str_rejects_unknown() {
        assert_eq!(AllowedAudioMime::from_mime_str("audio/flac"), None);
        assert_eq!(AllowedAudioMime::from_mime_str("image/png"), None);
        assert_eq!(AllowedAudioMime::from_mime_str(""), None);
    }

    #[test]
    fn as_str_round_trip() {
        for mime in [
            AllowedAudioMime::OggOpus,
            AllowedAudioMime::Mp3,
            AllowedAudioMime::Wav,
            AllowedAudioMime::M4a,
        ] {
            let s = mime.as_str();
            assert_eq!(
                AllowedAudioMime::from_mime_str(s),
                Some(mime),
                "round-trip failed for {s}"
            );
        }
    }

    #[test]
    fn file_extension_correct() {
        assert_eq!(AllowedAudioMime::OggOpus.file_extension(), "ogg");
        assert_eq!(AllowedAudioMime::Mp3.file_extension(), "mp3");
        assert_eq!(AllowedAudioMime::Wav.file_extension(), "wav");
        assert_eq!(AllowedAudioMime::M4a.file_extension(), "m4a");
    }

    // ── validate_audio_mime magic bytes (Task 2.2) ────────────

    #[test]
    fn validate_audio_mime_detects_ogg() {
        let bytes = b"OggS\x00\x02\x00\x00\x00\x00\x00\x00";
        assert_eq!(
            validate_audio_mime(None, bytes),
            Ok(AllowedAudioMime::OggOpus)
        );
    }

    #[test]
    fn validate_audio_mime_detects_mp3_id3() {
        let bytes = b"ID3\x04\x00\x00\x00\x00";
        assert_eq!(validate_audio_mime(None, bytes), Ok(AllowedAudioMime::Mp3));
    }

    #[test]
    fn validate_audio_mime_detects_mp3_sync_fb() {
        let bytes = [0xFF, 0xFB, 0x90, 0x00];
        assert_eq!(validate_audio_mime(None, &bytes), Ok(AllowedAudioMime::Mp3));
    }

    #[test]
    fn validate_audio_mime_detects_mp3_sync_f3() {
        let bytes = [0xFF, 0xF3, 0x90, 0x00];
        assert_eq!(validate_audio_mime(None, &bytes), Ok(AllowedAudioMime::Mp3));
    }

    #[test]
    fn validate_audio_mime_detects_mp3_sync_f2() {
        let bytes = [0xFF, 0xF2, 0x90, 0x00];
        assert_eq!(validate_audio_mime(None, &bytes), Ok(AllowedAudioMime::Mp3));
    }

    #[test]
    fn validate_audio_mime_detects_mp3_sync_e2() {
        // 0xE2 has top 3 bits set — valid MPEG sync but was previously rejected
        let bytes = [0xFF, 0xE2, 0x90, 0x00];
        assert_eq!(validate_audio_mime(None, &bytes), Ok(AllowedAudioMime::Mp3));
    }

    #[test]
    fn validate_audio_mime_rejects_reserved_mpeg_layer_bits() {
        // 0xE0 has layer bits = 0b00 (reserved) — must be rejected
        let bytes = [0xFF, 0xE0, 0x00, 0x00];
        assert!(validate_audio_mime(None, &bytes).is_err());
    }

    #[test]
    fn validate_audio_mime_rejects_adts_aac_header() {
        // 0xFF 0xF1 is ADTS AAC (layer bits = 0b00), not MP3
        let bytes = [0xFF, 0xF1, 0x00, 0x00];
        assert!(validate_audio_mime(None, &bytes).is_err());
    }

    #[test]
    fn validate_audio_mime_detects_mp3_layer3() {
        // 0xFF 0xFB = MPEG1 Layer3 (valid MP3)
        let bytes = [0xFF, 0xFB, 0x90, 0x00];
        assert_eq!(validate_audio_mime(None, &bytes), Ok(AllowedAudioMime::Mp3));
    }

    #[test]
    fn validate_audio_mime_rejects_mp3_sync_below_e0() {
        // 0xDF does NOT have top 3 bits set — invalid sync
        let bytes = [0xFF, 0xDF, 0x00, 0x00];
        assert_eq!(
            validate_audio_mime(None, &bytes),
            Err(AudioRejectionReason::MimeRejected)
        );
    }

    #[test]
    fn validate_audio_mime_detects_wav() {
        let mut bytes = vec![0u8; 12];
        bytes[0..4].copy_from_slice(b"RIFF");
        bytes[4..8].copy_from_slice(&[0x24, 0x08, 0x00, 0x00]); // file size
        bytes[8..12].copy_from_slice(b"WAVE");
        assert_eq!(validate_audio_mime(None, &bytes), Ok(AllowedAudioMime::Wav));
    }

    #[test]
    fn validate_audio_mime_detects_m4a() {
        let mut bytes = vec![0u8; 12];
        bytes[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x20]); // box size
        bytes[4..8].copy_from_slice(b"ftyp");
        bytes[8..12].copy_from_slice(b"M4A ");
        assert_eq!(validate_audio_mime(None, &bytes), Ok(AllowedAudioMime::M4a));
    }

    #[test]
    fn validate_audio_mime_rejects_unknown_bytes() {
        let bytes = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        assert_eq!(
            validate_audio_mime(Some("audio/ogg"), &bytes),
            Err(AudioRejectionReason::MimeRejected)
        );
    }

    #[test]
    fn validate_audio_mime_rejects_empty_bytes() {
        assert_eq!(
            validate_audio_mime(None, &[]),
            Err(AudioRejectionReason::MimeRejected)
        );
    }

    #[test]
    fn validate_audio_mime_rejects_flac_magic() {
        let bytes = b"fLaC\x00\x00\x00\x22";
        assert_eq!(
            validate_audio_mime(None, bytes),
            Err(AudioRejectionReason::MimeRejected)
        );
    }

    #[test]
    fn validate_audio_mime_rejects_midi() {
        let bytes = b"MThd\x00\x00\x00\x06";
        assert_eq!(
            validate_audio_mime(None, bytes),
            Err(AudioRejectionReason::MimeRejected)
        );
    }

    #[test]
    fn validate_audio_mime_ignores_declared_when_sniff_wins() {
        // Declared as MP3, but magic bytes are OGG
        let bytes = b"OggS\x00\x02\x00\x00\x00\x00\x00\x00";
        assert_eq!(
            validate_audio_mime(Some("audio/mpeg"), bytes),
            Ok(AllowedAudioMime::OggOpus)
        );
    }

    #[test]
    fn validate_audio_mime_ignores_declared_when_sniff_fails() {
        let bytes = [0x47, 0x49, 0x46]; // GIF magic — not audio
        assert_eq!(
            validate_audio_mime(Some("audio/ogg"), &bytes),
            Err(AudioRejectionReason::MimeRejected)
        );
    }

    #[test]
    fn validate_audio_mime_rejects_too_short_bytes() {
        assert_eq!(
            validate_audio_mime(None, &[0xFF]),
            Err(AudioRejectionReason::MimeRejected)
        );
    }

    // ── AudioRejectionReason Display (Task 2.3) ───────────────

    #[test]
    fn rejection_reason_display_strings() {
        assert_eq!(AudioRejectionReason::Disabled.to_string(), "disabled");
        assert_eq!(
            AudioRejectionReason::ChannelNotAllowed.to_string(),
            "channel_not_allowed"
        );
        assert_eq!(
            AudioRejectionReason::FetchFailed.to_string(),
            "fetch_failed"
        );
        assert_eq!(
            AudioRejectionReason::MimeRejected.to_string(),
            "mime_rejected"
        );
        assert_eq!(AudioRejectionReason::Oversize.to_string(), "oversize");
        assert_eq!(AudioRejectionReason::TooLong.to_string(), "too_long");
        assert_eq!(AudioRejectionReason::Corrupted.to_string(), "corrupted");
        assert_eq!(
            AudioRejectionReason::TranscriberUnavailable.to_string(),
            "transcriber_unavailable"
        );
        assert_eq!(
            AudioRejectionReason::TranscriptionFailed.to_string(),
            "transcription_failed"
        );
        assert_eq!(
            AudioRejectionReason::NoSpeechDetected.to_string(),
            "no_speech_detected"
        );
        assert_eq!(
            AudioRejectionReason::MultipleAudioParts.to_string(),
            "multiple_audio_parts"
        );
        assert_eq!(
            AudioRejectionReason::SystemError.to_string(),
            "system_error"
        );
    }

    // ── validate_audio_size ───────────────────────────────────

    #[test]
    fn validate_audio_size_accepts_within_limit() {
        assert!(validate_audio_size(1024, MAX_AUDIO_BYTES).is_ok());
        assert!(validate_audio_size(MAX_AUDIO_BYTES, MAX_AUDIO_BYTES).is_ok());
    }

    #[test]
    fn validate_audio_size_rejects_over_limit() {
        assert_eq!(
            validate_audio_size(MAX_AUDIO_BYTES + 1, MAX_AUDIO_BYTES),
            Err(AudioRejectionReason::Oversize)
        );
    }

    // ── validate_audio_duration ───────────────────────────────

    #[test]
    fn validate_audio_duration_accepts_within_limit() {
        assert!(validate_audio_duration(120, MAX_AUDIO_DURATION_SECS).is_ok());
        assert!(validate_audio_duration(MAX_AUDIO_DURATION_SECS, MAX_AUDIO_DURATION_SECS).is_ok());
    }

    #[test]
    fn validate_audio_duration_rejects_over_limit() {
        assert_eq!(
            validate_audio_duration(MAX_AUDIO_DURATION_SECS + 1, MAX_AUDIO_DURATION_SECS),
            Err(AudioRejectionReason::TooLong)
        );
    }

    // ── StagedAudio cleanup (Task 2.4) ────────────────────────

    #[test]
    fn staged_audio_cleanup_removes_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let tmp = dir.path().join("test_cleanup.ogg");
        std::fs::write(&tmp, b"fake audio").unwrap();

        let staged = StagedAudio {
            sha256: "abc123".into(),
            mime_type: AllowedAudioMime::OggOpus,
            byte_len: 10,
            duration_secs: Some(5.0),
            temp_path: tmp.clone(),
            channel_origin: "telegram".into(),
        };

        assert!(tmp.exists());
        staged.cleanup();
        assert!(!tmp.exists());
    }

    #[test]
    fn staged_audio_cleanup_noop_missing_file() {
        let staged = StagedAudio {
            sha256: "abc123".into(),
            mime_type: AllowedAudioMime::OggOpus,
            byte_len: 10,
            duration_secs: None,
            temp_path: PathBuf::from("/tmp/nonexistent_audio_test_xyz.ogg"),
            channel_origin: "telegram".into(),
        };
        // Should not panic on missing file
        staged.cleanup();
    }

    // ── AudioHistoryMeta (Task 2.4) ───────────────────────────

    #[test]
    fn audio_history_meta_from_staged() {
        let staged = StagedAudio {
            sha256: "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6".into(),
            mime_type: AllowedAudioMime::OggOpus,
            byte_len: 50_000,
            duration_secs: Some(15.0),
            temp_path: PathBuf::from("/tmp/test.ogg"),
            channel_origin: "telegram".into(),
        };

        let meta = AudioHistoryMeta::from_staged(&staged, "Hola mundo", Some("caption"));

        assert_eq!(meta.mime, "audio/ogg");
        assert_eq!(meta.sha256, "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6");
        assert_eq!(meta.byte_len, 50_000);
        assert_eq!(meta.duration_secs, Some(15.0));
        assert_eq!(meta.channel_origin, "telegram");
        assert_eq!(meta.transcription, "Hola mundo");
        assert_eq!(meta.caption, Some("caption".to_string()));
    }

    #[test]
    fn audio_history_meta_from_staged_no_caption() {
        let staged = StagedAudio {
            sha256: "deadbeef12345678".into(),
            mime_type: AllowedAudioMime::Mp3,
            byte_len: 1024,
            duration_secs: None,
            temp_path: PathBuf::from("/tmp/test.mp3"),
            channel_origin: "telegram".into(),
        };

        let meta = AudioHistoryMeta::from_staged(&staged, "Hello world", None);

        assert_eq!(meta.caption, None);
        assert_eq!(meta.transcription, "Hello world");
        assert_eq!(meta.duration_secs, None);
    }

    #[test]
    fn audio_history_meta_to_context_string_with_duration() {
        let meta = AudioHistoryMeta {
            mime: "audio/ogg".into(),
            sha256: "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6".into(),
            byte_len: 50_000,
            duration_secs: Some(45.0),
            channel_origin: "telegram".into(),
            transcription: "Hola, ¿cómo estás?".into(),
            caption: None,
        };

        let ctx = meta.to_context_string();
        assert!(ctx.starts_with("[Prior audio: audio/ogg"));
        assert!(ctx.contains(", 45s"));
        assert!(ctx.contains("Transcription: Hola, ¿cómo estás?"));
        assert!(ctx.ends_with(']'));
        // sha256 and byte_len should NOT be in model-facing context
        assert!(!ctx.contains("sha256"));
        assert!(!ctx.contains("50000 bytes"));
    }

    #[test]
    fn audio_history_meta_to_context_string_with_caption() {
        let meta = AudioHistoryMeta {
            mime: "audio/mpeg".into(),
            sha256: "deadbeef12345678".into(),
            byte_len: 1024,
            duration_secs: None,
            channel_origin: "telegram".into(),
            transcription: "Hello world".into(),
            caption: Some("translate this".into()),
        };

        let ctx = meta.to_context_string();
        assert!(ctx.contains("Caption: translate this"));
        assert!(ctx.contains("Transcription: Hello world"));
    }

    #[test]
    fn audio_history_meta_to_context_string_no_duration() {
        let meta = AudioHistoryMeta {
            mime: "audio/mpeg".into(),
            sha256: "deadbeef12345678".into(),
            byte_len: 1024,
            duration_secs: None,
            channel_origin: "telegram".into(),
            transcription: "Hello".into(),
            caption: None,
        };

        let ctx = meta.to_context_string();
        // When duration is None, should go straight to transcription
        assert!(ctx.starts_with("[Prior audio: audio/mpeg"));
        assert!(!ctx.contains("sha256"));
        assert!(!ctx.contains("1024 bytes"));
        assert!(ctx.contains("Transcription: Hello"));
        assert!(ctx.ends_with(']'));
    }

    #[test]
    fn audio_history_meta_to_context_string_truncates_long_transcription() {
        let long_text = "a".repeat(300);
        let meta = AudioHistoryMeta {
            mime: "audio/ogg".into(),
            sha256: "a1b2c3d4e5f6a7b8".into(),
            byte_len: 100,
            duration_secs: Some(10.0),
            channel_origin: "telegram".into(),
            transcription: long_text,
            caption: None,
        };

        let ctx = meta.to_context_string();
        // Transcription should be truncated to 200 chars
        let after_label = ctx.split("Transcription: ").nth(1).unwrap();
        let transcription_part = after_label.trim_end_matches(']');
        assert_eq!(transcription_part.len(), 200);
    }
}
