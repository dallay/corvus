use std::time::{Duration, SystemTime};

pub use corvus_traits::multimedia::{
    AllowedImageMime, ImageHistoryMeta, ImageTransportForm, StagedImage,
};
use futures_util::StreamExt;

/// Maximum image payload size (10 MiB).
pub const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

/// Hard ceiling for `max_image_bytes` config override (50 MiB).
/// Prevents operator misconfiguration from accepting arbitrarily large images.
pub const MAX_IMAGE_BYTES_CEILING: u64 = 52_428_800;

/// Default images allowed per turn when config omits the limit.
pub const DEFAULT_MAX_IMAGES_PER_TURN: usize = 4;

/// Hard ceiling for `max_images_per_turn` config override.
pub const MAX_IMAGES_PER_TURN_CEILING: usize = 8;

/// Default startup-only reaper threshold for stale staged images.
pub const DEFAULT_STAGED_IMAGE_REAPER_THRESHOLD_MINUTES: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StagedImageReaperReport {
    pub scanned_entries: usize,
    pub matched_files: usize,
    pub deleted_files: usize,
}

impl StagedImageReaperReport {
    fn record_scan(&mut self) {
        self.scanned_entries += 1;
    }

    fn record_match(&mut self) {
        self.matched_files += 1;
    }

    fn record_delete(&mut self) {
        self.deleted_files += 1;
    }
}

pub fn reap_startup_staged_images(threshold: Duration) -> StagedImageReaperReport {
    reap_startup_staged_images_in_dir(&std::env::temp_dir(), threshold)
}

fn reap_startup_staged_images_in_dir(
    dir: &std::path::Path,
    threshold: Duration,
) -> StagedImageReaperReport {
    reap_startup_staged_images_in_dir_at(dir, threshold, SystemTime::now())
}

fn reap_startup_staged_images_in_dir_at(
    dir: &std::path::Path,
    threshold: Duration,
    now: SystemTime,
) -> StagedImageReaperReport {
    reap_startup_staged_images_in_dir_at_with_remover(dir, threshold, now, |path| {
        std::fs::remove_file(path)
    })
}

fn reap_startup_staged_images_in_dir_at_with_remover<F>(
    dir: &std::path::Path,
    threshold: Duration,
    now: SystemTime,
    mut remove_file: F,
) -> StagedImageReaperReport
where
    F: FnMut(&std::path::Path) -> std::io::Result<()>,
{
    let Ok(entries) = std::fs::read_dir(dir) else {
        return StagedImageReaperReport::default();
    };

    let mut report = StagedImageReaperReport::default();

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };

        report.record_scan();

        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }

        let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !is_corvus_staged_image_file_name(&file_name) {
            continue;
        }
        report.record_match();

        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        let Ok(age) = now.duration_since(modified) else {
            continue;
        };
        if age <= threshold {
            continue;
        }

        match remove_file(&entry.path()) {
            Ok(()) => report.record_delete(),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {}
        }
    }

    report
}

fn is_corvus_staged_image_file_name(file_name: &str) -> bool {
    let Some((stem, ext)) = file_name.rsplit_once('.') else {
        return false;
    };
    if !matches!(ext, "jpg" | "png" | "webp") {
        return false;
    }

    if let Some(legacy_sha) = stem.strip_prefix("corvus-tg-img-") {
        return is_lower_hex(legacy_sha, 16);
    }

    let Some(remainder) = stem.strip_prefix("corvus-") else {
        return false;
    };
    let Some((channel, suffix)) = remainder.split_once("-img-") else {
        return false;
    };
    if channel.is_empty()
        || !channel
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
    {
        return false;
    }

    let Some((sha, nonce)) = suffix.split_once('-') else {
        return false;
    };
    if nonce.contains('-') {
        return false;
    }

    is_lower_hex(sha, 16) && is_lower_hex(nonce, 8)
}

fn is_lower_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// Reason an image turn was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImageRejectionReason {
    #[error("disabled")]
    Disabled,
    #[error("channel_not_allowed")]
    ChannelNotAllowed,
    #[error("missing_vision_route")]
    MissingVisionRoute,
    #[error("route_not_image_capable")]
    RouteNotImageCapable,
    #[error("fetch_failed")]
    FetchFailed,
    #[error("mime_rejected")]
    MimeRejected,
    #[error("oversize")]
    Oversize,
    #[error("too_many_images")]
    TooManyImages,
    #[error("provider_error")]
    ProviderError,
    #[error("channel_not_supported")]
    ChannelNotSupported,
}

/// Best-effort cleanup of the staged temp file.
pub fn cleanup_staged_image(staged: &StagedImage) {
    if staged.temp_path.exists() {
        if let Err(e) = std::fs::remove_file(&staged.temp_path) {
            tracing::warn!(
                "Failed to remove staged image {}: {e}",
                staged.temp_path.display()
            );
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
pub fn validate_image_count(
    count: usize,
    max_images_per_turn: usize,
) -> Result<(), ImageRejectionReason> {
    if count > max_images_per_turn {
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
    use std::fs;
    use std::io;
    use std::time::{Duration, SystemTime};
    use tempfile::TempDir;

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
        assert_eq!(
            ImageRejectionReason::ChannelNotSupported.to_string(),
            "channel_not_supported"
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
        assert!(validate_image_count(0, DEFAULT_MAX_IMAGES_PER_TURN).is_ok());
        assert!(validate_image_count(4, DEFAULT_MAX_IMAGES_PER_TURN).is_ok());
        assert!(validate_image_count(8, MAX_IMAGES_PER_TURN_CEILING).is_ok());
    }

    #[test]
    fn validate_image_count_rejects_over_limit() {
        assert_eq!(
            validate_image_count(5, DEFAULT_MAX_IMAGES_PER_TURN),
            Err(ImageRejectionReason::TooManyImages)
        );
        assert_eq!(
            validate_image_count(9, MAX_IMAGES_PER_TURN_CEILING),
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
        cleanup_staged_image(&staged);
        assert!(!tmp.exists());
    }

    #[test]
    fn staged_image_cleanup_noop_when_missing() {
        let staged = StagedImage {
            sha256: "abc".into(),
            mime_type: AllowedImageMime::Png,
            byte_len: 0,
            temp_path: std::path::PathBuf::from("/tmp/nonexistent_corvus_test_file"),
            transport_form: ImageTransportForm::InlineBytes,
            channel_origin: "test".into(),
        };
        // Should not panic
        cleanup_staged_image(&staged);
    }

    // ── Constants ─────────────────────────────────────────────

    #[test]
    fn constants_match_design() {
        assert_eq!(MAX_IMAGE_BYTES, 10 * 1024 * 1024);
        assert_eq!(DEFAULT_MAX_IMAGES_PER_TURN, 4);
        assert_eq!(MAX_IMAGES_PER_TURN_CEILING, 8);
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
        cleanup_staged_image(&staged);
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
        cleanup_staged_image(&staged);
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
        cleanup_staged_image(&staged);
    }

    #[test]
    fn staged_image_reaper_matches_current_and_legacy_names() {
        assert!(is_corvus_staged_image_file_name(
            "corvus-telegram-img-0123456789abcdef-89abcdef.jpg"
        ));
        assert!(is_corvus_staged_image_file_name(
            "corvus-whatsapp-img-fedcba9876543210-0123abcd.png"
        ));
        assert!(is_corvus_staged_image_file_name(
            "corvus-discord-img-a1b2c3d4e5f60718-deadbeef.webp"
        ));
        assert!(is_corvus_staged_image_file_name(
            "corvus-tg-img-0123456789abcdef.jpg"
        ));
    }

    #[test]
    fn staged_image_reaper_rejects_near_miss_names() {
        for invalid in [
            "corvus-telegram-img-0123456789abcde-89abcdef.jpg",
            "corvus-telegram-img-0123456789abcdef-89abcdeg.jpg",
            "corvus-telegram-img-0123456789abcdef-89abcdef.gif",
            "corvus-telegram-img-0123456789abcdef-89abcdef.jpg.tmp",
            "corvus-tg-img-0123456789abcdeg.jpg",
            "corvus-img-0123456789abcdef-89abcdef.jpg",
            "other-telegram-img-0123456789abcdef-89abcdef.jpg",
        ] {
            assert!(
                !is_corvus_staged_image_file_name(invalid),
                "expected '{invalid}' to be rejected"
            );
        }
    }

    #[test]
    fn staged_image_reaper_deletes_only_stale_matching_files() {
        let temp_dir = TempDir::new().unwrap();
        let stale = temp_dir
            .path()
            .join("corvus-telegram-img-0123456789abcdef-89abcdef.jpg");
        let legacy = temp_dir.path().join("corvus-tg-img-fedcba9876543210.png");
        let fresh = temp_dir
            .path()
            .join("corvus-discord-img-a1b2c3d4e5f60718-deadbeef.webp");
        let unrelated = temp_dir.path().join("notes.txt");

        fs::write(&stale, b"stale").unwrap();
        fs::write(&legacy, b"legacy").unwrap();
        fs::write(&unrelated, b"keep").unwrap();

        std::thread::sleep(Duration::from_millis(25));
        fs::write(&fresh, b"fresh").unwrap();

        let threshold = Duration::from_millis(10);
        let report =
            reap_startup_staged_images_in_dir_at(temp_dir.path(), threshold, SystemTime::now());

        assert_eq!(report.matched_files, 3);
        assert_eq!(report.deleted_files, 2);
        assert!(!stale.exists());
        assert!(!legacy.exists());
        assert!(fresh.exists());
        assert!(unrelated.exists());
    }

    #[test]
    fn staged_image_reaper_skips_future_timestamp_and_duplicate_execution() {
        let temp_dir = TempDir::new().unwrap();
        let candidate = temp_dir
            .path()
            .join("corvus-telegram-img-0123456789abcdef-89abcdef.jpg");
        fs::write(&candidate, b"candidate").unwrap();

        let threshold = Duration::from_secs(30);
        let future_skipped = reap_startup_staged_images_in_dir_at(
            temp_dir.path(),
            threshold,
            SystemTime::UNIX_EPOCH,
        );
        assert_eq!(future_skipped.deleted_files, 0);
        assert!(candidate.exists());

        std::thread::sleep(Duration::from_millis(20));

        let first = reap_startup_staged_images_in_dir_at(
            temp_dir.path(),
            Duration::from_millis(5),
            SystemTime::now(),
        );
        let second = reap_startup_staged_images_in_dir_at(
            temp_dir.path(),
            Duration::from_millis(5),
            SystemTime::now(),
        );

        assert_eq!(first.deleted_files, 1);
        assert_eq!(second.deleted_files, 0);
        assert!(!candidate.exists());
    }

    #[test]
    fn staged_image_reaper_treats_not_found_delete_race_as_non_fatal() {
        let temp_dir = TempDir::new().unwrap();
        let candidate = temp_dir
            .path()
            .join("corvus-telegram-img-0123456789abcdef-89abcdef.jpg");
        fs::write(&candidate, b"candidate").unwrap();
        std::thread::sleep(Duration::from_millis(20));

        let report = reap_startup_staged_images_in_dir_at_with_remover(
            temp_dir.path(),
            Duration::from_millis(5),
            SystemTime::now(),
            |_| Err(io::Error::from(io::ErrorKind::NotFound)),
        );

        assert_eq!(report.matched_files, 1);
        assert_eq!(report.deleted_files, 0);
        assert!(candidate.exists());
    }

    // ── ImageHistoryMeta (task 2.7) ───────────────────────────

    fn make_test_staged() -> StagedImage {
        StagedImage {
            sha256: "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2".into(),
            mime_type: AllowedImageMime::Jpeg,
            byte_len: 245_760,
            temp_path: std::path::PathBuf::from("/tmp/test.jpg"),
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
    fn image_history_meta_to_context_string_with_caption() {
        let meta = ImageHistoryMeta::from_staged(&make_test_staged(), Some("Hello world".into()));

        let ctx = meta.to_context_string();
        assert!(ctx.contains(". Caption: Hello world"));
        assert!(ctx.ends_with(']'));
    }

    #[test]
    fn image_history_meta_to_context_string_with_caption_and_description() {
        let mut meta =
            ImageHistoryMeta::from_staged(&make_test_staged(), Some("My caption".into()));
        meta.description = Some("A sunset photo".into());

        let ctx = meta.to_context_string();
        assert!(ctx.contains(". Description: A sunset photo"));
        assert!(ctx.contains(". Caption: My caption"));
        assert!(ctx.ends_with(']'));
    }

    #[test]
    fn image_history_meta_to_context_string_short_sha256() {
        let staged = StagedImage {
            sha256: "abcd1234".into(),
            mime_type: AllowedImageMime::Png,
            byte_len: 100,
            temp_path: std::path::PathBuf::from("/tmp/test.png"),
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
        cleanup_staged_image(&staged);
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

    #[test]
    fn image_history_meta_description_sanitized_and_truncated() {
        let mut meta = ImageHistoryMeta::from_staged(&make_test_staged(), None);
        // Description with newlines and length > 200
        let long_desc = format!("Line one\nLine two\r\nLine three {}", "x".repeat(250));
        meta.description = Some(long_desc);

        let ctx = meta.to_context_string();
        // Must not contain newlines
        assert!(!ctx.contains('\n'));
        assert!(!ctx.contains('\r'));
        // Description portion must be truncated to 200 chars
        // Extract the description substring
        let desc_start = ctx.find(". Description: ").unwrap() + ". Description: ".len();
        let desc_end = ctx[desc_start..]
            .find(']')
            .map(|i| i + desc_start)
            .or_else(|| ctx[desc_start..].find(". Caption:").map(|i| i + desc_start))
            .unwrap();
        let desc_text = &ctx[desc_start..desc_end];
        assert!(
            desc_text.len() <= 200,
            "description should be at most 200 chars, got {}",
            desc_text.len()
        );
        assert!(ctx.ends_with(']'));
    }
}
