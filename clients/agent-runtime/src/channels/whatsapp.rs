use super::media;
use super::traits::{Channel, ChannelMessage, ContentPart, SendMessage};
use async_trait::async_trait;
use uuid::Uuid;

/// `WhatsApp` channel — uses `WhatsApp` Business Cloud API
///
/// This channel operates in webhook mode (push-based) rather than polling.
/// Messages are received via the gateway's `/whatsapp` webhook endpoint.
/// The `listen` method here is a no-op placeholder; actual message handling
/// happens in the gateway when Meta sends webhook events.
pub struct WhatsAppChannel {
    access_token: String,
    endpoint_id: String,
    verify_token: String,
    allowed_numbers: Vec<String>,
    client: reqwest::Client,
}

/// Normalize a phone number to E.164 format (ensure leading `+`).
fn normalize_phone_number(from: &str) -> String {
    if from.starts_with('+') {
        from.to_string()
    } else {
        format!("+{from}")
    }
}

/// Mask a phone number, showing only the last 4 digits.
fn mask_phone(phone: &str) -> String {
    let count = phone.chars().count();
    if count <= 4 {
        return "****".to_string();
    }
    let visible: String = phone.chars().skip(count - 4).collect();
    format!("{}{visible}", "*".repeat(count - 4))
}

/// Extract canonical content parts from a WhatsApp message.
///
/// Returns `None` for unsupported message types (audio, video,
/// document, sticker, location, contacts, reaction, etc.).
/// For `type=text`, produces a single `Text` part.
/// For `type=image`, produces an `Image` part (plus a preceding
/// `Text` part when a caption is present).
fn extract_whatsapp_parts(
    msg: &serde_json::Value,
    from: &str,
) -> Option<(Vec<ContentPart>, String)> {
    let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match msg_type {
        "text" => {
            let body = msg
                .get("text")
                .and_then(|t| t.get("body"))
                .and_then(|b| b.as_str())
                .unwrap_or("")
                .to_string();
            if body.is_empty() {
                return None;
            }
            let parts = vec![ContentPart::Text { text: body.clone() }];
            Some((parts, body))
        }
        "image" => {
            let image_obj = msg.get("image")?;
            let media_id = image_obj
                .get("id")
                .and_then(|id| id.as_str())
                .unwrap_or("")
                .to_string();
            if media_id.is_empty() {
                return None;
            }
            let declared_mime = image_obj
                .get("mime_type")
                .and_then(|m| m.as_str())
                .map(ToString::to_string);
            let caption = image_obj
                .get("caption")
                .and_then(|c| c.as_str())
                .map(ToString::to_string)
                .filter(|c| !c.is_empty());

            let mut parts = Vec::new();
            if let Some(ref cap) = caption {
                parts.push(ContentPart::Text { text: cap.clone() });
            }
            parts.push(ContentPart::Image {
                channel_handle: media_id,
                source_channel: "whatsapp".to_string(),
                declared_mime,
                caption_text: caption.clone(),
                file_name: None,
                declared_bytes: None,
            });

            // Text projection: caption only (no placeholder)
            let content = caption.unwrap_or_default();
            Some((parts, content))
        }
        _ => {
            tracing::debug!(
                "WhatsApp: skipping unsupported message type \
                 '{}' from {}",
                msg_type,
                mask_phone(from),
            );
            None
        }
    }
}

/// Check whether a parts list contains at least one image.
fn has_image_part(parts: &[ContentPart]) -> bool {
    parts.iter().any(|p| matches!(p, ContentPart::Image { .. }))
}

/// Extract the timestamp from a WhatsApp message, falling back to current time.
fn extract_whatsapp_timestamp(msg: &serde_json::Value) -> u64 {
    msg.get("timestamp")
        .and_then(|t| t.as_str())
        .and_then(|t| t.parse::<u64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        })
}

impl WhatsAppChannel {
    pub fn new(
        access_token: String,
        endpoint_id: String,
        verify_token: String,
        allowed_numbers: Vec<String>,
    ) -> Self {
        Self {
            access_token,
            endpoint_id,
            verify_token,
            allowed_numbers,
            client: reqwest::Client::new(),
        }
    }

    /// Check if a phone number is allowed (E.164 format: +1234567890)
    fn is_number_allowed(&self, phone: &str) -> bool {
        self.allowed_numbers.iter().any(|n| n == "*" || n == phone)
    }

    /// Get the verify token for webhook verification
    pub fn verify_token(&self) -> &str {
        &self.verify_token
    }

    /// Fetch image bytes from WhatsApp Graph API, validate,
    /// stage to temp, and return a `StagedImage` or rejection.
    ///
    /// Two-step flow:
    /// 1. GET media metadata to obtain the download URL
    /// 2. GET the download URL to stream bytes
    pub async fn fetch_and_stage_image(
        &self,
        media_id: &str,
        declared_mime: Option<&str>,
    ) -> Result<media::StagedImage, media::ImageRejectionReason> {
        // 1. Resolve media id → download URL
        let meta_url = format!("https://graph.facebook.com/v21.0/{media_id}");
        let meta_resp = self
            .client
            .get(&meta_url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!(
                    "WhatsApp media metadata fetch failed \
                     for {}: {e}",
                    &media_id[..media_id.len().min(8)]
                );
                media::ImageRejectionReason::FetchFailed
            })?;

        if !meta_resp.status().is_success() {
            tracing::warn!("WhatsApp media metadata HTTP {}", meta_resp.status());
            return Err(media::ImageRejectionReason::FetchFailed);
        }

        let meta: serde_json::Value = meta_resp.json().await.map_err(|e| {
            tracing::warn!("WhatsApp media metadata parse error: {e}");
            media::ImageRejectionReason::FetchFailed
        })?;

        let download_url = meta
            .get("url")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                tracing::warn!("WhatsApp media metadata: missing url");
                media::ImageRejectionReason::FetchFailed
            })?;

        // 2. Download bytes with bearer auth + size limit
        let dl_resp = self
            .client
            .get(download_url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!("WhatsApp media download failed: {e}");
                media::ImageRejectionReason::FetchFailed
            })?;

        if !dl_resp.status().is_success() {
            tracing::warn!("WhatsApp media download HTTP {}", dl_resp.status());
            return Err(media::ImageRejectionReason::FetchFailed);
        }

        // Early reject via Content-Length
        if let Some(cl) = dl_resp.content_length() {
            media::validate_size(cl, media::MAX_IMAGE_BYTES)?;
        }

        let bytes = dl_resp.bytes().await.map_err(|e| {
            tracing::warn!("WhatsApp media download read error: {e}");
            media::ImageRejectionReason::FetchFailed
        })?;

        let byte_len = bytes.len() as u64;
        media::validate_size(byte_len, media::MAX_IMAGE_BYTES)?;

        // 3. Validate MIME via magic-byte sniffing
        let mime = media::validate_mime(declared_mime, &bytes)?;

        // 4. Stage to temp file and compute SHA-256
        use sha2::Digest;
        let sha256 = {
            let mut hasher = sha2::Sha256::new();
            hasher.update(&bytes);
            hex::encode(hasher.finalize())
        };

        let ext = match mime {
            media::AllowedImageMime::Jpeg => "jpg",
            media::AllowedImageMime::Png => "png",
            media::AllowedImageMime::Webp => "webp",
        };
        let temp_path =
            std::env::temp_dir().join(format!("corvus-wa-img-{}.{ext}", &sha256[..16],));

        tokio::fs::write(&temp_path, &bytes).await.map_err(|e| {
            tracing::warn!(
                "Failed to stage WhatsApp image to {}: {e}",
                temp_path.display()
            );
            media::ImageRejectionReason::FetchFailed
        })?;

        Ok(media::StagedImage {
            sha256,
            mime_type: mime,
            byte_len,
            temp_path,
            transport_form: media::ImageTransportForm::InlineBytes,
            channel_origin: "whatsapp".to_string(),
        })
    }

    /// Parse an incoming webhook payload from Meta and extract messages
    pub fn parse_webhook_payload(&self, payload: &serde_json::Value) -> Vec<ChannelMessage> {
        let mut messages = Vec::new();

        let Some(entries) = payload.get("entry").and_then(|e| e.as_array()) else {
            return messages;
        };

        for entry in entries {
            let Some(changes) = entry.get("changes").and_then(|c| c.as_array()) else {
                continue;
            };

            for change in changes {
                let Some(msgs) = change
                    .get("value")
                    .and_then(|v| v.get("messages"))
                    .and_then(|m| m.as_array())
                else {
                    continue;
                };

                for msg in msgs {
                    if let Some(channel_msg) = self.parse_single_whatsapp_message(msg) {
                        messages.push(channel_msg);
                    }
                }
            }
        }

        messages
    }

    /// Parse a single WhatsApp message JSON object into a
    /// `ChannelMessage`. Returns `None` if the message should be
    /// skipped (unauthorized sender, unsupported type, empty).
    fn parse_single_whatsapp_message(&self, msg: &serde_json::Value) -> Option<ChannelMessage> {
        let from = msg.get("from").and_then(|f| f.as_str())?;
        let normalized_from = normalize_phone_number(from);

        if !self.is_number_allowed(&normalized_from) {
            tracing::warn!(
                "WhatsApp: ignoring message from unauthorized \
                 number: {}. Add to allowed_numbers in \
                 config.toml, then run \
                 `corvus onboard --channels-only`.",
                mask_phone(&normalized_from),
            );
            return None;
        }

        let (parts, content) = extract_whatsapp_parts(msg, from)?;
        // Text-only messages with empty body are skipped
        if content.is_empty() && !has_image_part(&parts) {
            return None;
        }

        let timestamp = extract_whatsapp_timestamp(msg);

        Some(ChannelMessage {
            id: Uuid::new_v4().to_string(),
            reply_target: normalized_from.clone(),
            sender: normalized_from,
            content,
            channel: "whatsapp".to_string(),
            timestamp,
            parts,
        })
    }
}

#[async_trait]
impl Channel for WhatsAppChannel {
    fn name(&self) -> &str {
        "whatsapp"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        // WhatsApp Cloud API: POST to /v18.0/{phone_number_id}/messages
        let url = format!(
            "https://graph.facebook.com/v18.0/{}/messages",
            self.endpoint_id
        );

        // Normalize recipient (remove leading + if present for API)
        let to = message
            .recipient
            .strip_prefix('+')
            .unwrap_or(&message.recipient);

        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "text",
            "text": {
                "preview_url": false,
                "body": message.content
            }
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error_body = resp.text().await.unwrap_or_default();
            tracing::error!("WhatsApp send failed: {status} — {error_body}");
            anyhow::bail!("WhatsApp API error: {status}");
        }

        Ok(())
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        // WhatsApp uses webhooks (push-based), not polling.
        // Messages are received via the gateway's /whatsapp endpoint.
        // This method keeps the channel "alive" but doesn't actively poll.
        tracing::info!(
            "WhatsApp channel active (webhook mode). \
            Configure Meta webhook to POST to your gateway's /whatsapp endpoint."
        );

        // Keep the task alive — it will be cancelled when the channel shuts down
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    async fn health_check(&self) -> bool {
        // Check if we can reach the WhatsApp API
        let url = format!("https://graph.facebook.com/v18.0/{}", self.endpoint_id);

        self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_channel() -> WhatsAppChannel {
        WhatsAppChannel::new(
            "test-token".into(),
            "123456789".into(),
            "verify-me".into(),
            vec!["+1234567890".into()],
        )
    }

    #[test]
    fn whatsapp_channel_name() {
        let ch = make_channel();
        assert_eq!(ch.name(), "whatsapp");
    }

    #[test]
    fn whatsapp_verify_token() {
        let ch = make_channel();
        assert_eq!(ch.verify_token(), "verify-me");
    }

    #[test]
    fn whatsapp_number_allowed_exact() {
        let ch = make_channel();
        assert!(ch.is_number_allowed("+1234567890"));
        assert!(!ch.is_number_allowed("+9876543210"));
    }

    #[test]
    fn whatsapp_number_allowed_wildcard() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        assert!(ch.is_number_allowed("+1234567890"));
        assert!(ch.is_number_allowed("+9999999999"));
    }

    #[test]
    fn whatsapp_number_denied_empty() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec![]);
        assert!(!ch.is_number_allowed("+1234567890"));
    }

    #[test]
    fn whatsapp_parse_empty_payload() {
        let ch = make_channel();
        let payload = serde_json::json!({});
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_valid_text_message() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "id": "123",
                "changes": [{
                    "value": {
                        "messaging_product": "whatsapp",
                        "metadata": {
                            "display_phone_number": "15551234567",
                            "phone_number_id": "123456789"
                        },
                        "messages": [{
                            "from": "1234567890",
                            "id": "wamid.xxx",
                            "timestamp": "1699999999",
                            "type": "text",
                            "text": {
                                "body": "Hello Corvus!"
                            }
                        }]
                    },
                    "field": "messages"
                }]
            }]
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, "+1234567890");
        assert_eq!(msgs[0].content, "Hello Corvus!");
        assert_eq!(msgs[0].channel, "whatsapp");
        assert_eq!(msgs[0].timestamp, 1_699_999_999);
    }

    #[test]
    fn whatsapp_parse_unauthorized_number() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "9999999999",
                            "timestamp": "1699999999",
                            "type": "text",
                            "text": { "body": "Spam" }
                        }]
                    }
                }]
            }]
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty(), "Unauthorized numbers should be filtered");
    }

    #[test]
    fn whatsapp_parse_image_message_produces_image_part() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "1234567890",
                            "timestamp": "1699999999",
                            "type": "image",
                            "image": {
                                "id": "img123",
                                "mime_type": "image/jpeg"
                            }
                        }]
                    }
                }]
            }]
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].parts.len(), 1);
        match &msgs[0].parts[0] {
            ContentPart::Image {
                channel_handle,
                source_channel,
                declared_mime,
                caption_text,
                ..
            } => {
                assert_eq!(channel_handle, "img123");
                assert_eq!(source_channel, "whatsapp");
                assert_eq!(declared_mime.as_deref(), Some("image/jpeg"));
                assert!(caption_text.is_none());
            }
            ContentPart::Text { .. } => panic!("expected Image part"),
        }
        // Image-only: content is empty (no placeholder)
        assert!(msgs[0].content.is_empty());
    }

    #[test]
    fn whatsapp_parse_multiple_messages() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [
                            { "from": "111", "timestamp": "1", "type": "text", "text": { "body": "First" } },
                            { "from": "222", "timestamp": "2", "type": "text", "text": { "body": "Second" } }
                        ]
                    }
                }]
            }]
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "First");
        assert_eq!(msgs[1].content, "Second");
    }

    #[test]
    fn whatsapp_parse_normalizes_phone_with_plus() {
        let ch = WhatsAppChannel::new(
            "tok".into(),
            "123".into(),
            "ver".into(),
            vec!["+1234567890".into()],
        );
        // API sends without +, but we normalize to +
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "1234567890",
                            "timestamp": "1",
                            "type": "text",
                            "text": { "body": "Hi" }
                        }]
                    }
                }]
            }]
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, "+1234567890");
    }

    #[test]
    fn whatsapp_empty_text_skipped() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "text",
                            "text": { "body": "" }
                        }]
                    }
                }]
            }]
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    // ══════════════════════════════════════════════════════════
    // EDGE CASES — Comprehensive coverage
    // ══════════════════════════════════════════════════════════

    #[test]
    fn whatsapp_parse_missing_entry_array() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "object": "whatsapp_business_account"
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_entry_not_array() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "entry": "not_an_array"
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_missing_changes_array() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "entry": [{ "id": "123" }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_changes_not_array() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "entry": [{
                "changes": "not_an_array"
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_missing_value() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{ "field": "messages" }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_missing_messages_array() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "metadata": {}
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_messages_not_array() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": "not_an_array"
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_missing_from_field() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "timestamp": "1",
                            "type": "text",
                            "text": { "body": "No sender" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty(), "Messages without 'from' should be skipped");
    }

    #[test]
    fn whatsapp_parse_missing_text_body() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "text",
                            "text": {}
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(
            msgs.is_empty(),
            "Messages with empty text object should be skipped"
        );
    }

    #[test]
    fn whatsapp_parse_null_text_body() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "text",
                            "text": { "body": null }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty(), "Messages with null body should be skipped");
    }

    #[test]
    fn whatsapp_parse_invalid_timestamp_uses_current() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "not_a_number",
                            "type": "text",
                            "text": { "body": "Hello" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        // Timestamp should be current time (non-zero)
        assert!(msgs[0].timestamp > 0);
    }

    #[test]
    fn whatsapp_parse_missing_timestamp_uses_current() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "type": "text",
                            "text": { "body": "Hello" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].timestamp > 0);
    }

    #[test]
    fn whatsapp_parse_multiple_entries() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [
                {
                    "changes": [{
                        "value": {
                            "messages": [{
                                "from": "111",
                                "timestamp": "1",
                                "type": "text",
                                "text": { "body": "Entry 1" }
                            }]
                        }
                    }]
                },
                {
                    "changes": [{
                        "value": {
                            "messages": [{
                                "from": "222",
                                "timestamp": "2",
                                "type": "text",
                                "text": { "body": "Entry 2" }
                            }]
                        }
                    }]
                }
            ]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "Entry 1");
        assert_eq!(msgs[1].content, "Entry 2");
    }

    #[test]
    fn whatsapp_parse_multiple_changes() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [
                    {
                        "value": {
                            "messages": [{
                                "from": "111",
                                "timestamp": "1",
                                "type": "text",
                                "text": { "body": "Change 1" }
                            }]
                        }
                    },
                    {
                        "value": {
                            "messages": [{
                                "from": "222",
                                "timestamp": "2",
                                "type": "text",
                                "text": { "body": "Change 2" }
                            }]
                        }
                    }
                ]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "Change 1");
        assert_eq!(msgs[1].content, "Change 2");
    }

    #[test]
    fn whatsapp_parse_status_update_ignored() {
        // Status updates have "statuses" instead of "messages"
        let ch = make_channel();
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "statuses": [{
                            "id": "wamid.xxx",
                            "status": "delivered",
                            "timestamp": "1699999999"
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty(), "Status updates should be ignored");
    }

    #[test]
    fn whatsapp_parse_audio_message_skipped() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "audio",
                            "audio": { "id": "audio123", "mime_type": "audio/ogg" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_video_message_skipped() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "video",
                            "video": { "id": "video123" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_document_message_skipped() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "document",
                            "document": { "id": "doc123", "filename": "file.pdf" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_sticker_message_skipped() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "sticker",
                            "sticker": { "id": "sticker123" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_location_message_skipped() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "location",
                            "location": { "latitude": 40.7128, "longitude": -74.0060 }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_contacts_message_skipped() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "contacts",
                            "contacts": [{ "name": { "formatted_name": "John" } }]
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_reaction_message_skipped() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "reaction",
                            "reaction": { "message_id": "wamid.xxx", "emoji": "👍" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_mixed_authorized_unauthorized() {
        let ch = WhatsAppChannel::new(
            "tok".into(),
            "123".into(),
            "ver".into(),
            vec!["+1111111111".into()],
        );
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [
                            { "from": "1111111111", "timestamp": "1", "type": "text", "text": { "body": "Allowed" } },
                            { "from": "9999999999", "timestamp": "2", "type": "text", "text": { "body": "Blocked" } },
                            { "from": "1111111111", "timestamp": "3", "type": "text", "text": { "body": "Also allowed" } }
                        ]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "Allowed");
        assert_eq!(msgs[1].content, "Also allowed");
    }

    #[test]
    fn whatsapp_parse_unicode_message() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "text",
                            "text": { "body": "Hello 👋 世界 🌍 مرحبا" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "Hello 👋 世界 🌍 مرحبا");
    }

    #[test]
    fn whatsapp_parse_very_long_message() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let long_text = "A".repeat(10_000);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "text",
                            "text": { "body": long_text }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content.len(), 10_000);
    }

    #[test]
    fn whatsapp_parse_whitespace_only_message_preserved() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "text",
                            "text": { "body": "   " }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        // Whitespace-only is NOT empty, so it passes through
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "   ");
    }

    #[test]
    fn whatsapp_number_allowed_multiple_numbers() {
        let ch = WhatsAppChannel::new(
            "tok".into(),
            "123".into(),
            "ver".into(),
            vec![
                "+1111111111".into(),
                "+2222222222".into(),
                "+3333333333".into(),
            ],
        );
        assert!(ch.is_number_allowed("+1111111111"));
        assert!(ch.is_number_allowed("+2222222222"));
        assert!(ch.is_number_allowed("+3333333333"));
        assert!(!ch.is_number_allowed("+4444444444"));
    }

    #[test]
    fn whatsapp_number_allowed_case_sensitive() {
        // Phone numbers should be exact match
        let ch = WhatsAppChannel::new(
            "tok".into(),
            "123".into(),
            "ver".into(),
            vec!["+1234567890".into()],
        );
        assert!(ch.is_number_allowed("+1234567890"));
        // Different number should not match
        assert!(!ch.is_number_allowed("+1234567891"));
    }

    #[test]
    fn whatsapp_parse_phone_already_has_plus() {
        let ch = WhatsAppChannel::new(
            "tok".into(),
            "123".into(),
            "ver".into(),
            vec!["+1234567890".into()],
        );
        // If API sends with +, we should still handle it
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "+1234567890",
                            "timestamp": "1",
                            "type": "text",
                            "text": { "body": "Hi" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, "+1234567890");
    }

    #[test]
    fn whatsapp_channel_fields_stored_correctly() {
        let ch = WhatsAppChannel::new(
            "my-access-token".into(),
            "phone-id-123".into(),
            "my-verify-token".into(),
            vec!["+111".into(), "+222".into()],
        );
        assert_eq!(ch.verify_token(), "my-verify-token");
        assert!(ch.is_number_allowed("+111"));
        assert!(ch.is_number_allowed("+222"));
        assert!(!ch.is_number_allowed("+333"));
    }

    #[test]
    fn whatsapp_parse_empty_messages_array() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": []
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_empty_entry_array() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "entry": []
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_empty_changes_array() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "entry": [{
                "changes": []
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_newlines_preserved() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "text",
                            "text": { "body": "Line 1\nLine 2\nLine 3" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "Line 1\nLine 2\nLine 3");
    }

    // ── mask_phone ────────────────────────────────────────────

    #[test]
    fn mask_phone_normal_number() {
        assert_eq!(mask_phone("1234567890"), "******7890");
    }

    #[test]
    fn mask_phone_short_number() {
        assert_eq!(mask_phone("1234"), "****");
    }

    #[test]
    fn mask_phone_with_country_code() {
        // "+15551234567" is 12 chars, last 4 visible = "4567", 8 stars
        assert_eq!(mask_phone("+15551234567"), "********4567");
    }

    #[test]
    fn mask_phone_empty_string() {
        assert_eq!(mask_phone(""), "****");
    }

    #[test]
    fn mask_phone_exactly_four_chars() {
        assert_eq!(mask_phone("abcd"), "****");
    }

    #[test]
    fn mask_phone_five_chars() {
        assert_eq!(mask_phone("12345"), "*2345");
    }

    // ── normalize_phone_number ───────────────────────────────

    #[test]
    fn normalize_phone_adds_plus() {
        assert_eq!(normalize_phone_number("1234567890"), "+1234567890");
    }

    #[test]
    fn normalize_phone_already_has_plus() {
        assert_eq!(normalize_phone_number("+1234567890"), "+1234567890");
    }

    // ── extract_whatsapp_timestamp ───────────────────────────

    #[test]
    fn extract_timestamp_valid() {
        let msg = serde_json::json!({"timestamp": "1699999999"});
        assert_eq!(extract_whatsapp_timestamp(&msg), 1_699_999_999);
    }

    #[test]
    fn extract_timestamp_missing_falls_back() {
        let msg = serde_json::json!({});
        assert!(extract_whatsapp_timestamp(&msg) > 0);
    }

    #[test]
    fn extract_timestamp_invalid_falls_back() {
        let msg = serde_json::json!({"timestamp": "not_a_number"});
        assert!(extract_whatsapp_timestamp(&msg) > 0);
    }

    #[test]
    fn whatsapp_parse_special_characters() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "text",
                            "text": { "body": "<script>alert('xss')</script> & \"quotes\" 'apostrophe'" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(
            msgs[0].content,
            "<script>alert('xss')</script> & \"quotes\" 'apostrophe'"
        );
    }

    // ══════════════════════════════════════════════════════════
    // MULTIMODAL PARSING — Task 3.5 (WhatsApp portion)
    // ══════════════════════════════════════════════════════════

    #[test]
    fn whatsapp_text_message_produces_text_part() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "text",
                            "text": { "body": "Hello world" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].parts.len(), 1);
        match &msgs[0].parts[0] {
            ContentPart::Text { text } => {
                assert_eq!(text, "Hello world");
            }
            ContentPart::Image { .. } => panic!("expected Text part"),
        }
        assert_eq!(msgs[0].content, "Hello world");
    }

    #[test]
    fn text_only_whatsapp_regression_remains_text_only() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "2",
                            "type": "text",
                            "text": { "body": "plain text still works" }
                        }]
                    }
                }]
            }]
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "plain text still works");
        assert!(!msgs[0].has_image_parts());
        assert_eq!(msgs[0].parts.len(), 1);
    }

    #[test]
    fn whatsapp_image_with_caption_produces_text_and_image() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "image",
                            "image": {
                                "id": "media_456",
                                "mime_type": "image/png",
                                "caption": "Check this out"
                            }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].parts.len(), 2);

        // First part: caption as Text
        match &msgs[0].parts[0] {
            ContentPart::Text { text } => {
                assert_eq!(text, "Check this out");
            }
            ContentPart::Image { .. } => panic!("expected Text part first"),
        }

        // Second part: Image with caption_text set
        match &msgs[0].parts[1] {
            ContentPart::Image {
                channel_handle,
                source_channel,
                declared_mime,
                caption_text,
                file_name,
                declared_bytes,
            } => {
                assert_eq!(channel_handle, "media_456");
                assert_eq!(source_channel, "whatsapp");
                assert_eq!(declared_mime.as_deref(), Some("image/png"));
                assert_eq!(caption_text.as_deref(), Some("Check this out"));
                assert!(file_name.is_none());
                assert!(declared_bytes.is_none());
            }
            ContentPart::Text { .. } => panic!("expected Image part second"),
        }

        // Content is the caption text
        assert_eq!(msgs[0].content, "Check this out");
    }

    #[test]
    fn whatsapp_image_without_mime_still_parses() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "image",
                            "image": { "id": "no_mime_img" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        match &msgs[0].parts[0] {
            ContentPart::Image { declared_mime, .. } => {
                assert!(declared_mime.is_none());
            }
            ContentPart::Text { .. } => panic!("expected Image part"),
        }
    }

    #[test]
    fn whatsapp_image_missing_id_skipped() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "image",
                            "image": { "mime_type": "image/jpeg" }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty(), "Image without media id should be skipped");
    }

    #[test]
    fn whatsapp_unsupported_types_still_skipped() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        for msg_type in [
            "audio", "video", "document", "sticker", "location", "contacts", "reaction",
        ] {
            let payload = serde_json::json!({
                "entry": [{
                    "changes": [{
                        "value": {
                            "messages": [{
                                "from": "111",
                                "timestamp": "1",
                                "type": msg_type,
                                msg_type: { "id": "x" }
                            }]
                        }
                    }]
                }]
            });
            let msgs = ch.parse_webhook_payload(&payload);
            assert!(msgs.is_empty(), "type={msg_type} should be skipped");
        }
    }

    #[test]
    fn whatsapp_image_empty_caption_not_emitted() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "111",
                            "timestamp": "1",
                            "type": "image",
                            "image": {
                                "id": "img_empty_cap",
                                "caption": ""
                            }
                        }]
                    }
                }]
            }]
        });
        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        // Only Image part, no Text part for empty caption
        assert_eq!(msgs[0].parts.len(), 1);
        assert!(matches!(
            &msgs[0].parts[0],
            ContentPart::Image { caption_text, .. }
            if caption_text.is_none()
        ));
    }
}
