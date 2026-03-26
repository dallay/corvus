use std::fmt;
use std::path::PathBuf;

/// Maximum image payload size (10 MiB).
pub const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

/// Maximum images allowed per turn for MVP.
pub const MAX_IMAGES_PER_TURN: usize = 1;

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

/// Reason an image turn was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageRejectionReason {
    Disabled,
    ChannelNotAllowed,
    MissingVisionRoute,
    RouteNotImageCapable,
    FetchFailed,
    MimeRejected,
    Oversize,
    TooManyImages,
    ProviderError,
}

impl fmt::Display for ImageRejectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            Self::Disabled => "disabled",
            Self::ChannelNotAllowed => "channel_not_allowed",
            Self::MissingVisionRoute => "missing_vision_route",
            Self::RouteNotImageCapable => "route_not_image_capable",
            Self::FetchFailed => "fetch_failed",
            Self::MimeRejected => "mime_rejected",
            Self::Oversize => "oversize",
            Self::TooManyImages => "too_many_images",
            Self::ProviderError => "provider_error",
        };
        f.write_str(code)
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

impl StagedImage {
    /// Best-effort cleanup of the staged temp file.
    pub fn cleanup(&self) {
        if self.temp_path.exists() {
            if let Err(e) = std::fs::remove_file(&self.temp_path) {
                tracing::warn!(
                    "Failed to remove staged image {}: {e}",
                    self.temp_path.display()
                );
            }
        }
    }
}

/// Validate image MIME type by sniffing magic bytes first, then
/// falling back to the declared MIME only if sniffing passes.
pub fn validate_mime(
    declared: Option<&str>,
    sniffed_bytes: &[u8],
) -> Result<AllowedImageMime, ImageRejectionReason> {
    // JPEG: FF D8 FF
    if sniffed_bytes.len() >= 3
        && sniffed_bytes[0] == 0xFF
        && sniffed_bytes[1] == 0xD8
        && sniffed_bytes[2] == 0xFF
    {
        return Ok(AllowedImageMime::Jpeg);
    }

    // PNG: 89 50 4E 47
    if sniffed_bytes.len() >= 4
        && sniffed_bytes[0] == 0x89
        && sniffed_bytes[1] == 0x50
        && sniffed_bytes[2] == 0x4E
        && sniffed_bytes[3] == 0x47
    {
        return Ok(AllowedImageMime::Png);
    }

    // WebP: RIFF....WEBP (bytes 0-3 = RIFF, bytes 8-11 = WEBP)
    if sniffed_bytes.len() >= 12
        && &sniffed_bytes[0..4] == b"RIFF"
        && &sniffed_bytes[8..12] == b"WEBP"
    {
        return Ok(AllowedImageMime::Webp);
    }

    // If magic bytes didn't match any known type but declared MIME
    // is one of our allowed types, still reject — sniffing takes
    // precedence for security.
    let _ = declared;
    Err(ImageRejectionReason::MimeRejected)
}

/// Validate that the image size is within the allowed limit.
pub fn validate_size(byte_len: u64, max_bytes: u64) -> Result<(), ImageRejectionReason> {
    if byte_len > max_bytes {
        Err(ImageRejectionReason::Oversize)
    } else {
        Ok(())
    }
}

/// Validate that the image count does not exceed the per-turn limit.
pub fn validate_image_count(count: usize) -> Result<(), ImageRejectionReason> {
    if count > MAX_IMAGES_PER_TURN {
        Err(ImageRejectionReason::TooManyImages)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── AllowedImageMime ──────────────────────────────────────

    #[test]
    fn mime_from_str_known_types() {
        assert_eq!(
            AllowedImageMime::from_mime_str("image/jpeg"),
            Some(AllowedImageMime::Jpeg)
        );
        assert_eq!(
            AllowedImageMime::from_mime_str("image/png"),
            Some(AllowedImageMime::Png)
        );
        assert_eq!(
            AllowedImageMime::from_mime_str("image/webp"),
            Some(AllowedImageMime::Webp)
        );
    }

    #[test]
    fn mime_from_str_rejects_unknown() {
        assert_eq!(AllowedImageMime::from_mime_str("image/gif"), None);
        assert_eq!(AllowedImageMime::from_mime_str("text/plain"), None);
        assert_eq!(AllowedImageMime::from_mime_str(""), None);
    }

    #[test]
    fn mime_as_str_roundtrips() {
        for mime in [
            AllowedImageMime::Jpeg,
            AllowedImageMime::Png,
            AllowedImageMime::Webp,
        ] {
            assert_eq!(AllowedImageMime::from_mime_str(mime.as_str()), Some(mime));
        }
    }

    // ── ImageRejectionReason Display ──────────────────────────

    #[test]
    fn rejection_reason_display_uses_snake_case() {
        assert_eq!(ImageRejectionReason::Disabled.to_string(), "disabled");
        assert_eq!(
            ImageRejectionReason::ChannelNotAllowed.to_string(),
            "channel_not_allowed"
        );
        assert_eq!(
            ImageRejectionReason::MissingVisionRoute.to_string(),
            "missing_vision_route"
        );
        assert_eq!(
            ImageRejectionReason::RouteNotImageCapable.to_string(),
            "route_not_image_capable"
        );
        assert_eq!(
            ImageRejectionReason::FetchFailed.to_string(),
            "fetch_failed"
        );
        assert_eq!(
            ImageRejectionReason::MimeRejected.to_string(),
            "mime_rejected"
        );
        assert_eq!(ImageRejectionReason::Oversize.to_string(), "oversize");
        assert_eq!(
            ImageRejectionReason::TooManyImages.to_string(),
            "too_many_images"
        );
        assert_eq!(
            ImageRejectionReason::ProviderError.to_string(),
            "provider_error"
        );
    }

    // ── validate_mime ─────────────────────────────────────────

    #[test]
    fn validate_mime_detects_jpeg() {
        let bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00];
        assert_eq!(validate_mime(None, &bytes), Ok(AllowedImageMime::Jpeg));
    }

    #[test]
    fn validate_mime_detects_png() {
        let bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
        assert_eq!(validate_mime(None, &bytes), Ok(AllowedImageMime::Png));
    }

    #[test]
    fn validate_mime_detects_webp() {
        let mut bytes = vec![0u8; 12];
        bytes[0..4].copy_from_slice(b"RIFF");
        bytes[8..12].copy_from_slice(b"WEBP");
        assert_eq!(validate_mime(None, &bytes), Ok(AllowedImageMime::Webp));
    }

    #[test]
    fn validate_mime_rejects_unknown_bytes() {
        let bytes = [0x00, 0x01, 0x02, 0x03];
        assert_eq!(
            validate_mime(Some("image/jpeg"), &bytes),
            Err(ImageRejectionReason::MimeRejected)
        );
    }

    #[test]
    fn validate_mime_rejects_empty_bytes() {
        assert_eq!(
            validate_mime(None, &[]),
            Err(ImageRejectionReason::MimeRejected)
        );
    }

    #[test]
    fn validate_mime_ignores_declared_when_sniff_fails() {
        let bytes = [0x47, 0x49, 0x46]; // GIF magic
        assert_eq!(
            validate_mime(Some("image/png"), &bytes),
            Err(ImageRejectionReason::MimeRejected)
        );
    }

    // ── validate_size ─────────────────────────────────────────

    #[test]
    fn validate_size_accepts_within_limit() {
        assert!(validate_size(1024, MAX_IMAGE_BYTES).is_ok());
        assert!(validate_size(MAX_IMAGE_BYTES, MAX_IMAGE_BYTES).is_ok());
    }

    #[test]
    fn validate_size_rejects_over_limit() {
        assert_eq!(
            validate_size(MAX_IMAGE_BYTES + 1, MAX_IMAGE_BYTES),
            Err(ImageRejectionReason::Oversize)
        );
    }

    // ── validate_image_count ──────────────────────────────────

    #[test]
    fn validate_image_count_accepts_within_limit() {
        assert!(validate_image_count(0).is_ok());
        assert!(validate_image_count(1).is_ok());
    }

    #[test]
    fn validate_image_count_rejects_over_limit() {
        assert_eq!(
            validate_image_count(2),
            Err(ImageRejectionReason::TooManyImages)
        );
    }

    // ── StagedImage cleanup ───────────────────────────────────

    #[test]
    fn staged_image_cleanup_removes_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let tmp = dir.path().join("test_cleanup.jpg");
        std::fs::write(&tmp, b"fake").unwrap();

        let staged = StagedImage {
            sha256: "abc".into(),
            mime_type: AllowedImageMime::Jpeg,
            byte_len: 4,
            temp_path: tmp.clone(),
            transport_form: ImageTransportForm::InlineBytes,
            channel_origin: "test".into(),
        };

        assert!(tmp.exists());
        staged.cleanup();
        assert!(!tmp.exists());
    }

    #[test]
    fn staged_image_cleanup_noop_when_missing() {
        let staged = StagedImage {
            sha256: "abc".into(),
            mime_type: AllowedImageMime::Png,
            byte_len: 0,
            temp_path: PathBuf::from("/tmp/nonexistent_corvus_test_file"),
            transport_form: ImageTransportForm::InlineBytes,
            channel_origin: "test".into(),
        };
        // Should not panic
        staged.cleanup();
    }

    // ── Constants ─────────────────────────────────────────────

    #[test]
    fn constants_match_design() {
        assert_eq!(MAX_IMAGE_BYTES, 10 * 1024 * 1024);
        assert_eq!(MAX_IMAGES_PER_TURN, 1);
    }
}
