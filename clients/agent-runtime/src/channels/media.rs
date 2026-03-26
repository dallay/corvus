use std::fmt;
use std::path::PathBuf;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

/// Maximum image payload size (10 MiB).
pub const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

/// Hard ceiling for `max_image_bytes` config override (50 MiB).
/// Prevents operator misconfiguration from accepting arbitrarily large images.
pub const MAX_IMAGE_BYTES_CEILING: u64 = 52_428_800;

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
            let _ = write!(s, ". Description: {desc}");
        }
        s.push(']');
        s
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

    // PNG: 89 50 4E 47 0D 0A 1A 0A (full 8-byte signature)
    if sniffed_bytes.len() >= 8
        && sniffed_bytes[0] == 0x89
        && sniffed_bytes[1] == 0x50
        && sniffed_bytes[2] == 0x4E
        && sniffed_bytes[3] == 0x47
        && sniffed_bytes[4] == 0x0D
        && sniffed_bytes[5] == 0x0A
        && sniffed_bytes[6] == 0x1A
        && sniffed_bytes[7] == 0x0A
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

/// Shared post-HTTP-response flow for image staging.
///
/// Takes an already-sent `reqwest::Response` and:
/// 1. Checks HTTP status
/// 2. Early-rejects via Content-Length
/// 3. Streams bytes with per-chunk size validation
/// 4. Validates MIME via magic-byte sniffing
/// 5. Computes SHA-256 hash
/// 6. Writes to a temp file with `channel_prefix` and a UUID nonce
///
/// Each channel still performs its own HTTP request (different auth).
/// This helper only consumes the `Response`.
pub async fn stream_validate_and_stage(
    response: reqwest::Response,
    declared_mime: Option<&str>,
    channel_prefix: &str,
    sanitize_url: &str,
    max_bytes: u64,
) -> Result<StagedImage, ImageRejectionReason> {
    // 1. Check HTTP status
    if !response.status().is_success() {
        tracing::warn!("{channel_prefix} image download HTTP {}", response.status());
        return Err(ImageRejectionReason::FetchFailed);
    }

    // 2. Early reject via Content-Length
    if let Some(cl) = response.content_length() {
        validate_size(cl, max_bytes)?;
    }

    // 3. Stream bytes with per-chunk size validation
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| {
            let sanitized = format!("{e}").replace(sanitize_url, "[URL]");
            tracing::warn!("{channel_prefix} image download stream read error: {sanitized}");
            ImageRejectionReason::FetchFailed
        })?;
        bytes.extend_from_slice(&chunk);
        validate_size(bytes.len() as u64, max_bytes)?;
    }
    let byte_len = bytes.len() as u64;

    // 4. Validate MIME via magic-byte sniffing
    let mime = validate_mime(declared_mime, &bytes)?;

    // 5. Compute SHA-256 hash
    use sha2::Digest;
    let sha256 = {
        let mut hasher = sha2::Sha256::new();
        hasher.update(&bytes);
        hex::encode(hasher.finalize())
    };

    // 6. Write to temp file with channel prefix and nonce
    let ext = match mime {
        AllowedImageMime::Jpeg => "jpg",
        AllowedImageMime::Png => "png",
        AllowedImageMime::Webp => "webp",
    };
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let temp_path = std::env::temp_dir().join(format!(
        "corvus-{channel_prefix}-img-{}-{}.{ext}",
        &sha256[..16],
        &nonce[..8]
    ));

    tokio::fs::write(&temp_path, &bytes).await.map_err(|e| {
        tracing::warn!(
            "Failed to stage {channel_prefix} image to {}: {e}",
            temp_path.display()
        );
        ImageRejectionReason::FetchFailed
    })?;

    Ok(StagedImage {
        sha256,
        mime_type: mime,
        byte_len,
        temp_path,
        transport_form: ImageTransportForm::InlineBytes,
        channel_origin: channel_prefix.to_string(),
    })
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
        let bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
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

    // ── stream_validate_and_stage ─────────────────────────────

    /// Build a mock HTTP response with the given status, body, and
    /// optional Content-Type header.
    fn mock_response(status: u16, body: &[u8], content_type: Option<&str>) -> reqwest::Response {
        let mut builder = http::Response::builder().status(status);
        if let Some(ct) = content_type {
            builder = builder.header("content-type", ct);
        }
        let resp = builder
            .body(body.to_vec())
            .expect("failed to build mock response");
        reqwest::Response::from(resp)
    }

    #[tokio::test]
    async fn stage_valid_jpeg_succeeds() {
        // Minimal JPEG: FF D8 FF + padding to be a real-ish payload
        let mut body = vec![0xFF, 0xD8, 0xFF, 0xE0];
        body.extend_from_slice(&[0u8; 100]);

        let resp = mock_response(200, &body, Some("image/jpeg"));
        let result = stream_validate_and_stage(
            resp,
            Some("image/jpeg"),
            "test",
            "https://example.com/img.jpg",
            MAX_IMAGE_BYTES,
        )
        .await;

        let staged = result.expect("should succeed for valid JPEG");
        assert_eq!(staged.mime_type, AllowedImageMime::Jpeg);
        assert_eq!(staged.byte_len, body.len() as u64);
        assert_eq!(staged.channel_origin, "test");
        assert!(staged.temp_path.exists());
        assert!(staged
            .temp_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("corvus-test-img-"));
        assert!(staged.temp_path.extension().unwrap().to_str().unwrap() == "jpg");
        // Verify SHA-256 is a 64-char hex string
        assert_eq!(staged.sha256.len(), 64);
        assert!(staged.sha256.chars().all(|c| c.is_ascii_hexdigit()));
        staged.cleanup();
    }

    #[tokio::test]
    async fn stage_valid_png_succeeds() {
        let mut body = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        body.extend_from_slice(&[0u8; 50]);

        let resp = mock_response(200, &body, Some("image/png"));
        let result =
            stream_validate_and_stage(resp, Some("image/png"), "ch", "http://x", MAX_IMAGE_BYTES)
                .await;

        let staged = result.expect("should succeed for valid PNG");
        assert_eq!(staged.mime_type, AllowedImageMime::Png);
        assert_eq!(staged.channel_origin, "ch");
        assert!(staged.temp_path.extension().unwrap().to_str().unwrap() == "png");
        staged.cleanup();
    }

    #[tokio::test]
    async fn stage_rejects_non_success_status() {
        let resp = mock_response(404, b"not found", None);
        let result =
            stream_validate_and_stage(resp, None, "test", "http://x", MAX_IMAGE_BYTES).await;
        assert!(
            matches!(result, Err(ImageRejectionReason::FetchFailed)),
            "expected FetchFailed, got {result:?}"
        );
    }

    #[tokio::test]
    async fn stage_rejects_unknown_mime() {
        // GIF magic bytes — not in the allowed list
        let body = b"GIF89a\x00\x00\x00\x00";
        let resp = mock_response(200, body, Some("image/gif"));
        let result =
            stream_validate_and_stage(resp, Some("image/gif"), "test", "http://x", MAX_IMAGE_BYTES)
                .await;
        assert!(
            matches!(result, Err(ImageRejectionReason::MimeRejected)),
            "expected MimeRejected, got {result:?}"
        );
    }

    #[tokio::test]
    async fn stage_rejects_oversize_body() {
        // Body exceeds MAX_IMAGE_BYTES — streaming validation
        // must catch it even without a Content-Length header.
        let mut body = vec![0xFF, 0xD8, 0xFF, 0xE0];
        #[allow(clippy::cast_possible_truncation)]
        body.resize(MAX_IMAGE_BYTES as usize + 1, 0x00);

        let resp = mock_response(200, &body, Some("image/jpeg"));
        let result =
            stream_validate_and_stage(resp, None, "test", "http://x", MAX_IMAGE_BYTES).await;
        assert!(
            matches!(result, Err(ImageRejectionReason::Oversize)),
            "expected Oversize, got {result:?}"
        );
    }

    #[tokio::test]
    async fn stage_channel_prefix_in_filename() {
        let mut body = vec![0xFF, 0xD8, 0xFF, 0xE0];
        body.extend_from_slice(&[0u8; 20]);

        let resp = mock_response(200, &body, None);
        let result =
            stream_validate_and_stage(resp, Some("image/jpeg"), "wa", "http://x", MAX_IMAGE_BYTES)
                .await;

        let staged = result.expect("should succeed");
        let fname = staged
            .temp_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(
            fname.starts_with("corvus-wa-img-"),
            "filename should contain channel prefix: {fname}"
        );
        staged.cleanup();
    }

    // ── ImageHistoryMeta (task 2.7) ───────────────────────────

    fn make_test_staged() -> StagedImage {
        StagedImage {
            sha256: "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2".into(),
            mime_type: AllowedImageMime::Jpeg,
            byte_len: 245_760,
            temp_path: PathBuf::from("/tmp/test.jpg"),
            transport_form: ImageTransportForm::InlineBytes,
            channel_origin: "telegram".into(),
        }
    }

    #[test]
    fn image_history_meta_from_staged_maps_fields() {
        let staged = make_test_staged();
        let meta = ImageHistoryMeta::from_staged(&staged, Some("My garden".into()));

        assert_eq!(meta.mime, "image/jpeg");
        assert_eq!(meta.sha256, staged.sha256);
        assert_eq!(meta.byte_len, 245_760);
        assert_eq!(meta.channel_origin, "telegram");
        assert_eq!(meta.caption, Some("My garden".into()));
        assert!(meta.description.is_none());
    }

    #[test]
    fn image_history_meta_from_staged_no_caption() {
        let staged = make_test_staged();
        let meta = ImageHistoryMeta::from_staged(&staged, None);

        assert!(meta.caption.is_none());
        assert!(meta.description.is_none());
    }

    #[test]
    fn image_history_meta_to_context_string_with_description() {
        let mut meta = ImageHistoryMeta::from_staged(&make_test_staged(), None);
        meta.description = Some("A photo of a garden".into());

        let ctx = meta.to_context_string();
        assert!(ctx.starts_with("[Prior image: image/jpeg, 245760 bytes, sha256:a1b2c3d4e5f6a7b8"));
        assert!(ctx.contains(". Description: A photo of a garden"));
        assert!(ctx.ends_with(']'));
    }

    #[test]
    fn image_history_meta_to_context_string_without_description() {
        let meta = ImageHistoryMeta::from_staged(&make_test_staged(), None);

        let ctx = meta.to_context_string();
        assert!(ctx.starts_with("[Prior image: image/jpeg, 245760 bytes, sha256:a1b2c3d4e5f6a7b8"));
        assert!(!ctx.contains("Description"));
        assert!(ctx.ends_with(']'));
    }

    #[test]
    fn image_history_meta_to_context_string_short_sha256() {
        let staged = StagedImage {
            sha256: "abcd1234".into(),
            mime_type: AllowedImageMime::Png,
            byte_len: 100,
            temp_path: PathBuf::from("/tmp/test.png"),
            transport_form: ImageTransportForm::InlineBytes,
            channel_origin: "test".into(),
        };
        let meta = ImageHistoryMeta::from_staged(&staged, None);

        let ctx = meta.to_context_string();
        assert!(ctx.contains("sha256:abcd1234"));
    }

    #[test]
    fn image_history_meta_serde_roundtrip() {
        let meta = ImageHistoryMeta::from_staged(&make_test_staged(), Some("caption".into()));
        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: ImageHistoryMeta = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.mime, meta.mime);
        assert_eq!(deserialized.sha256, meta.sha256);
        assert_eq!(deserialized.byte_len, meta.byte_len);
        assert_eq!(deserialized.channel_origin, meta.channel_origin);
        assert_eq!(deserialized.caption, meta.caption);
        assert_eq!(deserialized.description, meta.description);
    }

    // ── stream_validate_and_stage custom max_bytes (task 2.9) ─

    #[tokio::test]
    async fn stage_custom_max_bytes_accepts_within_custom_limit() {
        // Body larger than default MAX_IMAGE_BYTES but within custom limit
        let custom_limit: u64 = 20 * 1024 * 1024; // 20 MiB
        let mut body = vec![0xFF, 0xD8, 0xFF, 0xE0];
        #[allow(clippy::cast_possible_truncation)]
        body.resize(MAX_IMAGE_BYTES as usize + 100, 0x00); // slightly above default

        let resp = mock_response(200, &body, Some("image/jpeg"));
        let result =
            stream_validate_and_stage(resp, Some("image/jpeg"), "test", "http://x", custom_limit)
                .await;

        let staged = result.expect("should succeed with custom higher limit");
        assert_eq!(staged.byte_len, body.len() as u64);
        staged.cleanup();
    }

    #[tokio::test]
    async fn stage_custom_max_bytes_rejects_above_custom_limit() {
        let custom_limit: u64 = 5 * 1024 * 1024; // 5 MiB
        let mut body = vec![0xFF, 0xD8, 0xFF, 0xE0];
        #[allow(clippy::cast_possible_truncation)]
        body.resize(custom_limit as usize + 1, 0x00);

        let resp = mock_response(200, &body, Some("image/jpeg"));
        let result =
            stream_validate_and_stage(resp, Some("image/jpeg"), "test", "http://x", custom_limit)
                .await;
        assert!(
            matches!(result, Err(ImageRejectionReason::Oversize)),
            "expected Oversize, got {result:?}"
        );
    }

    #[tokio::test]
    async fn stage_custom_max_bytes_lower_than_default_rejects() {
        let custom_limit: u64 = 1024; // 1 KiB — very small
        let mut body = vec![0xFF, 0xD8, 0xFF, 0xE0];
        body.extend_from_slice(&[0u8; 2048]); // 2 KiB+ total

        let resp = mock_response(200, &body, Some("image/jpeg"));
        let result =
            stream_validate_and_stage(resp, Some("image/jpeg"), "test", "http://x", custom_limit)
                .await;
        assert!(
            matches!(result, Err(ImageRejectionReason::Oversize)),
            "expected Oversize, got {result:?}"
        );
    }

    // ── Constants (task 2.4) ──────────────────────────────────

    #[test]
    fn max_image_bytes_ceiling_is_50_mib() {
        assert_eq!(MAX_IMAGE_BYTES_CEILING, 52_428_800);
        assert_eq!(MAX_IMAGE_BYTES_CEILING, 50 * 1024 * 1024);
    }
}
