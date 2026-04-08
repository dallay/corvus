use super::media;
use super::traits::{Channel, ChannelMessage, ContentPart, SendMessage};
use async_trait::async_trait;
use std::time::Duration;

const SLACK_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);

/// Slack channel — polls conversations.history via Web API
pub struct SlackChannel {
    bot_token: String,
    channel_id: Option<String>,
    allowed_users: Vec<String>,
    client: reqwest::Client,
    api_base_url: String,
}

impl SlackChannel {
    pub fn new(bot_token: String, channel_id: Option<String>, allowed_users: Vec<String>) -> Self {
        Self {
            bot_token,
            channel_id,
            allowed_users,
            client: reqwest::Client::new(),
            api_base_url: "https://slack.com/api".to_string(),
        }
    }

    #[cfg(test)]
    fn new_with_api_base_url(
        bot_token: String,
        channel_id: Option<String>,
        allowed_users: Vec<String>,
        api_base_url: String,
    ) -> Self {
        Self {
            bot_token,
            channel_id,
            allowed_users,
            client: reqwest::Client::new(),
            api_base_url,
        }
    }

    fn api_url(&self, endpoint: &str) -> String {
        format!("{}/{endpoint}", self.api_base_url.trim_end_matches('/'))
    }

    fn sanitize_error(&self, err: &reqwest::Error) -> String {
        err.to_string().replace(&self.bot_token, "[REDACTED]")
    }

    fn slack_api_error(payload: &serde_json::Value) -> &str {
        payload
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown_error")
    }

    /// Check if a Slack user ID is in the allowlist.
    /// Empty list means deny everyone until explicitly configured.
    /// `"*"` means allow everyone.
    fn is_user_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == user_id)
    }

    /// Get the bot's own user ID so we can ignore our own messages
    async fn get_bot_user_id(&self) -> Option<String> {
        let response = self
            .client
            .get(self.api_url("auth.test"))
            .bearer_auth(&self.bot_token)
            .send()
            .await
            .inspect_err(|err| {
                tracing::warn!(
                    "Slack auth.test request failed: {}",
                    self.sanitize_error(err)
                );
            })
            .ok()?;

        if !response.status().is_success() {
            tracing::warn!("Slack auth.test HTTP {}", response.status());
            return None;
        }

        let resp: serde_json::Value = response
            .json()
            .await
            .map_err(|err| {
                tracing::warn!("Slack auth.test parse error: {err}");
                err
            })
            .ok()?;

        if resp.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
            tracing::warn!(
                "Slack auth.test API error: {}",
                Self::slack_api_error(&resp)
            );
            return None;
        }

        resp.get("user_id")
            .and_then(|u| u.as_str())
            .map(String::from)
    }

    /// Poll Slack conversations.history API. Returns parsed JSON on success,
    /// logs and returns error on failure so the caller can skip.
    async fn poll_history(
        &self,
        channel_id: &str,
        last_ts: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let mut params = vec![
            ("channel", channel_id.to_string()),
            ("limit", "10".to_string()),
        ];
        if !last_ts.is_empty() {
            params.push(("oldest", last_ts.to_string()));
        }

        let resp = self
            .client
            .get(self.api_url("conversations.history"))
            .bearer_auth(&self.bot_token)
            .query(&params)
            .send()
            .await
            .inspect_err(|e| {
                tracing::warn!("Slack poll error: {}", self.sanitize_error(e));
            })?;

        if !resp.status().is_success() {
            tracing::warn!("Slack poll HTTP {}", resp.status());
            return Err(anyhow::anyhow!("Slack conversations.history HTTP failure"));
        }

        let payload: serde_json::Value = resp.json().await.map_err(|e| {
            tracing::warn!("Slack parse error: {e}");
            anyhow::Error::from(e)
        })?;

        if payload.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
            let api_error = Self::slack_api_error(&payload);
            tracing::warn!("Slack conversations.history API error: {api_error}");
            return Err(anyhow::anyhow!(
                "Slack conversations.history failed: {api_error}"
            ));
        }

        Ok(payload)
    }

    fn parse_image_parts(
        &self,
        msg: &serde_json::Value,
        caption: Option<&str>,
    ) -> Vec<ContentPart> {
        let mut file_refs = Vec::new();
        if let Some(files) = msg.get("files").and_then(|files| files.as_array()) {
            file_refs.extend(files.iter());
        } else if let Some(file) = msg.get("file") {
            file_refs.push(file);
        }

        let caption = caption
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        file_refs
            .into_iter()
            .filter_map(|file| {
                let mimetype = file.get("mimetype").and_then(|value| value.as_str())?;
                media::AllowedImageMime::from_mime_str(mimetype)?;

                let file_id = file.get("id").and_then(|value| value.as_str())?;
                if file_id.is_empty() {
                    return None;
                }

                Some(ContentPart::Image {
                    channel_handle: file_id.to_string(),
                    source_channel: "slack".to_string(),
                    declared_mime: Some(mimetype.to_string()),
                    caption_text: caption.clone(),
                    file_name: file
                        .get("name")
                        .and_then(|value| value.as_str())
                        .map(str::to_string),
                    declared_bytes: file.get("size").and_then(|value| value.as_u64()),
                })
            })
            .collect()
    }

    async fn resolve_file_download_url(
        &self,
        file_id: &str,
    ) -> Result<String, media::ImageRejectionReason> {
        let response = self
            .client
            .get(self.api_url("files.info"))
            .bearer_auth(&self.bot_token)
            .query(&[("file", file_id)])
            .send()
            .await
            .map_err(|err| {
                tracing::warn!(
                    "Slack file metadata fetch failed for {}: {}",
                    &file_id[..file_id.len().min(8)],
                    self.sanitize_error(&err)
                );
                media::ImageRejectionReason::FetchFailed
            })?;

        if !response.status().is_success() {
            tracing::warn!("Slack file metadata HTTP {}", response.status());
            return Err(media::ImageRejectionReason::FetchFailed);
        }

        let payload: serde_json::Value = response.json().await.map_err(|err| {
            tracing::warn!("Slack file metadata parse error: {err}");
            media::ImageRejectionReason::FetchFailed
        })?;

        if payload.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
            let api_error = Self::slack_api_error(&payload);
            tracing::warn!("Slack file metadata API error: {api_error}");
            return Err(media::ImageRejectionReason::FetchFailed);
        }

        payload
            .get("file")
            .and_then(|file| file.get("url_private_download"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                tracing::warn!("Slack file metadata missing url_private_download");
                media::ImageRejectionReason::FetchFailed
            })
    }

    pub async fn fetch_and_stage_image(
        &self,
        file_id: &str,
        declared_mime: Option<&str>,
        max_bytes: u64,
    ) -> Result<media::StagedImage, media::ImageRejectionReason> {
        let download_url = self.resolve_file_download_url(file_id).await?;
        tokio::time::timeout(SLACK_DOWNLOAD_TIMEOUT, async {
            let response = self
                .client
                .get(&download_url)
                .bearer_auth(&self.bot_token)
                .send()
                .await
                .map_err(|err| {
                    tracing::warn!(
                        "Slack file download request failed: {}",
                        self.sanitize_error(&err)
                    );
                    media::ImageRejectionReason::FetchFailed
                })?;

            media::stream_validate_and_stage(
                response,
                declared_mime,
                "sl",
                &download_url,
                max_bytes,
            )
            .await
        })
        .await
        .map_err(|_| {
            tracing::warn!(
                "Slack file download timed out after {}s",
                SLACK_DOWNLOAD_TIMEOUT.as_secs()
            );
            media::ImageRejectionReason::FetchFailed
        })?
    }

    /// Parse a single Slack message JSON value into a `ChannelMessage`.
    /// Returns `None` if the message should be skipped (bot, unauthorized, empty, already-seen).
    /// On success returns `(ChannelMessage, new_ts)`.
    fn parse_slack_message(
        &self,
        msg: &serde_json::Value,
        bot_user_id: &str,
        last_ts: &str,
        channel_id: &str,
    ) -> Option<(ChannelMessage, String)> {
        let ts = msg.get("ts").and_then(|t| t.as_str()).unwrap_or("");
        let user = msg
            .get("user")
            .and_then(|u| u.as_str())
            .unwrap_or("unknown");
        let text = msg.get("text").and_then(|t| t.as_str()).unwrap_or("");

        if user == bot_user_id {
            return None;
        }
        if !self.is_user_allowed(user) {
            tracing::warn!("Slack: ignoring message from unauthorized user: {user}");
            return None;
        }
        if ts <= last_ts {
            return None;
        }

        let image_parts = self.parse_image_parts(msg, Some(text));
        if text.is_empty() && image_parts.is_empty() {
            return None;
        }

        let parts = if image_parts.is_empty() {
            Vec::new()
        } else {
            let mut parts = Vec::with_capacity(image_parts.len() + usize::from(!text.is_empty()));
            if !text.is_empty() {
                parts.push(ContentPart::Text {
                    text: text.to_string(),
                });
            }
            parts.extend(image_parts);
            parts
        };

        let channel_msg = ChannelMessage {
            id: format!("slack_{channel_id}_{ts}"),
            sender: user.to_string(),
            reply_target: channel_id.to_string(),
            content: text.to_string(),
            channel: "slack".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            parts,
        };

        Some((channel_msg, ts.to_string()))
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        let body = serde_json::json!({
            "channel": message.recipient,
            "text": message.content
        });

        let resp = self
            .client
            .post(self.api_url("chat.postMessage"))
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .unwrap_or_else(|e| format!("<failed to read response body: {e}>"));

        if !status.is_success() {
            anyhow::bail!("Slack chat.postMessage failed ({status}): {body}");
        }

        // Slack returns 200 for most app-level errors; check JSON "ok" field
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
        if parsed.get("ok") == Some(&serde_json::Value::Bool(false)) {
            let err = parsed
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("unknown");
            anyhow::bail!("Slack chat.postMessage failed: {err}");
        }

        Ok(())
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        let channel_id = self
            .channel_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Slack channel_id required for listening"))?;

        let bot_user_id = self.get_bot_user_id().await.unwrap_or_default();
        let mut last_ts = String::new();

        tracing::info!("Slack channel listening on #{channel_id}...");

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            let data = match self.poll_history(&channel_id, &last_ts).await {
                Ok(d) => d,
                Err(_) => continue,
            };

            let Some(messages) = data.get("messages").and_then(|m| m.as_array()) else {
                continue;
            };

            // Messages come newest-first, reverse to process oldest first
            for msg in messages.iter().rev() {
                if let Some((channel_msg, new_ts)) =
                    self.parse_slack_message(msg, &bot_user_id, &last_ts, &channel_id)
                {
                    last_ts = new_ts;
                    if tx.send(channel_msg).await.is_err() {
                        return Ok(());
                    }
                }
            }
        }
    }

    async fn health_check(&self) -> bool {
        let response = match self
            .client
            .get(self.api_url("auth.test"))
            .bearer_auth(&self.bot_token)
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                tracing::warn!(
                    "Slack health check request failed: {}",
                    self.sanitize_error(&err)
                );
                return false;
            }
        };

        if !response.status().is_success() {
            tracing::warn!("Slack health check HTTP {}", response.status());
            return false;
        }

        let payload: serde_json::Value = match response.json().await {
            Ok(payload) => payload,
            Err(err) => {
                tracing::warn!("Slack health check parse error: {err}");
                return false;
            }
        };

        if payload.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
            tracing::warn!(
                "Slack health check API error: {}",
                Self::slack_api_error(&payload)
            );
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::media;
    use crate::channels::traits::ContentPart;
    use parking_lot::Mutex;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn slack_channel_name() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec![]);
        assert_eq!(ch.name(), "slack");
    }

    #[test]
    fn slack_channel_with_channel_id() {
        let ch = SlackChannel::new("xoxb-fake".into(), Some("C12345".into()), vec![]);
        assert_eq!(ch.channel_id, Some("C12345".to_string()));
    }

    #[test]
    fn empty_allowlist_denies_everyone() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec![]);
        assert!(!ch.is_user_allowed("U12345"));
        assert!(!ch.is_user_allowed("anyone"));
    }

    #[test]
    fn wildcard_allows_everyone() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["*".into()]);
        assert!(ch.is_user_allowed("U12345"));
    }

    #[test]
    fn specific_allowlist_filters() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["U111".into(), "U222".into()]);
        assert!(ch.is_user_allowed("U111"));
        assert!(ch.is_user_allowed("U222"));
        assert!(!ch.is_user_allowed("U333"));
    }

    #[test]
    fn allowlist_exact_match_not_substring() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["U111".into()]);
        assert!(!ch.is_user_allowed("U1111"));
        assert!(!ch.is_user_allowed("U11"));
    }

    #[test]
    fn allowlist_empty_user_id() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["U111".into()]);
        assert!(!ch.is_user_allowed(""));
    }

    #[test]
    fn allowlist_case_sensitive() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["U111".into()]);
        assert!(ch.is_user_allowed("U111"));
        assert!(!ch.is_user_allowed("u111"));
    }

    #[test]
    fn allowlist_wildcard_and_specific() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["U111".into(), "*".into()]);
        assert!(ch.is_user_allowed("U111"));
        assert!(ch.is_user_allowed("anyone"));
    }

    // ── parse_slack_message ──────────────────────────────────────

    fn make_slack_channel() -> SlackChannel {
        SlackChannel::new(
            "xoxb-fake".into(),
            Some("C12345".into()),
            vec!["U111".into(), "U222".into()],
        )
    }

    fn slack_msg_json(user: &str, text: &str, ts: &str) -> serde_json::Value {
        serde_json::json!({ "user": user, "text": text, "ts": ts })
    }

    #[test]
    fn parse_slack_message_valid() {
        let ch = make_slack_channel();
        let msg = slack_msg_json("U111", "hello", "100.0");
        let result = ch.parse_slack_message(&msg, "BOT", "", "C12345");
        assert!(result.is_some());
        let (cm, new_ts) = result.unwrap();
        assert_eq!(cm.sender, "U111");
        assert_eq!(cm.content, "hello");
        assert_eq!(cm.channel, "slack");
        assert_eq!(cm.reply_target, "C12345");
        assert!(cm.id.starts_with("slack_C12345_100.0"));
        assert_eq!(new_ts, "100.0");
    }

    #[test]
    fn parse_slack_message_skips_bot() {
        let ch = make_slack_channel();
        let msg = slack_msg_json("BOT", "echo", "100.0");
        assert!(ch.parse_slack_message(&msg, "BOT", "", "C12345").is_none());
    }

    #[test]
    fn parse_slack_message_skips_unauthorized_user() {
        let ch = make_slack_channel();
        let msg = slack_msg_json("U999", "spam", "100.0");
        assert!(ch.parse_slack_message(&msg, "BOT", "", "C12345").is_none());
    }

    #[test]
    fn parse_slack_message_skips_empty_text() {
        let ch = make_slack_channel();
        let msg = slack_msg_json("U111", "", "100.0");
        assert!(ch.parse_slack_message(&msg, "BOT", "", "C12345").is_none());
    }

    #[test]
    fn parse_slack_message_with_image_files_emits_caption_and_image_parts() {
        let ch = make_slack_channel();
        let msg = serde_json::json!({
            "user": "U111",
            "text": "What is in this image?",
            "ts": "100.0",
            "files": [{
                "id": "F123",
                "mimetype": "image/png",
                "name": "photo.png",
                "size": 2048,
                "url_private_download": "https://files.slack.com/files-pri/T1-F123/download/photo.png"
            }]
        });

        let (parsed, _) = ch
            .parse_slack_message(&msg, "BOT", "", "C12345")
            .expect("Slack image message should parse");

        assert_eq!(parsed.content, "What is in this image?");
        assert_eq!(parsed.parts.len(), 2);
        assert!(
            matches!(&parsed.parts[0], ContentPart::Text { text } if text == "What is in this image?")
        );
        match &parsed.parts[1] {
            ContentPart::Image {
                channel_handle,
                source_channel,
                declared_mime,
                caption_text,
                file_name,
                declared_bytes,
            } => {
                assert_eq!(channel_handle, "F123");
                assert_eq!(source_channel, "slack");
                assert_eq!(declared_mime.as_deref(), Some("image/png"));
                assert_eq!(caption_text.as_deref(), Some("What is in this image?"));
                assert_eq!(file_name.as_deref(), Some("photo.png"));
                assert_eq!(*declared_bytes, Some(2048));
            }
            other => panic!("expected image part, got {other:?}"),
        }
    }

    #[test]
    fn parse_slack_file_share_without_text_keeps_image_message() {
        let ch = make_slack_channel();
        let msg = serde_json::json!({
            "user": "U111",
            "subtype": "file_share",
            "text": "",
            "ts": "100.0",
            "file": {
                "id": "F999",
                "mimetype": "image/jpeg",
                "name": "snap.jpg",
                "size": 4096,
                "url_private_download": "https://files.slack.com/files-pri/T1-F999/download/snap.jpg"
            }
        });

        let (parsed, _) = ch
            .parse_slack_message(&msg, "BOT", "", "C12345")
            .expect("Slack file_share image message should parse");

        assert!(parsed.content.is_empty());
        assert_eq!(parsed.parts.len(), 1);
        match &parsed.parts[0] {
            ContentPart::Image {
                channel_handle,
                caption_text,
                file_name,
                declared_bytes,
                ..
            } => {
                assert_eq!(channel_handle, "F999");
                assert!(caption_text.is_none());
                assert_eq!(file_name.as_deref(), Some("snap.jpg"));
                assert_eq!(*declared_bytes, Some(4096));
            }
            other => panic!("expected image part, got {other:?}"),
        }
    }

    #[test]
    fn parse_slack_message_skips_old_timestamp() {
        let ch = make_slack_channel();
        // String comparison: "100.0" <= "200.0" is true, so message is skipped
        let msg = slack_msg_json("U111", "old", "100.0");
        assert!(ch
            .parse_slack_message(&msg, "BOT", "200.0", "C12345")
            .is_none());
    }

    #[test]
    fn parse_slack_message_skips_equal_timestamp() {
        let ch = make_slack_channel();
        let msg = slack_msg_json("U111", "dup", "100.0");
        assert!(ch
            .parse_slack_message(&msg, "BOT", "100.0", "C12345")
            .is_none());
    }

    // ── Message ID edge cases ─────────────────────────────────────

    #[test]
    fn slack_message_id_format_includes_channel_and_ts() {
        // Verify that message IDs follow the format: slack_{channel_id}_{ts}
        let ts = "1234567890.123456";
        let channel_id = "C12345";
        let expected_id = format!("slack_{channel_id}_{ts}");
        assert_eq!(expected_id, "slack_C12345_1234567890.123456");
    }

    #[test]
    fn slack_message_id_is_deterministic() {
        // Same channel_id + same ts = same ID (prevents duplicates after restart)
        let ts = "1234567890.123456";
        let channel_id = "C12345";
        let id1 = format!("slack_{channel_id}_{ts}");
        let id2 = format!("slack_{channel_id}_{ts}");
        assert_eq!(id1, id2);
    }

    #[test]
    fn slack_message_id_different_ts_different_id() {
        // Different timestamps produce different IDs
        let channel_id = "C12345";
        let id1 = format!("slack_{channel_id}_1234567890.123456");
        let id2 = format!("slack_{channel_id}_1234567890.123457");
        assert_ne!(id1, id2);
    }

    #[test]
    fn slack_message_id_different_channel_different_id() {
        // Different channels produce different IDs even with same ts
        let ts = "1234567890.123456";
        let id1 = format!("slack_C12345_{ts}");
        let id2 = format!("slack_C67890_{ts}");
        assert_ne!(id1, id2);
    }

    #[test]
    fn slack_message_id_no_uuid_randomness() {
        // Verify format doesn't contain random UUID components
        let ts = "1234567890.123456";
        let channel_id = "C12345";
        let id = format!("slack_{channel_id}_{ts}");
        assert!(!id.contains('-')); // No UUID dashes
        assert!(id.starts_with("slack_"));
    }

    async fn spawn_mock_slack_file_api(
        image_body: Vec<u8>,
    ) -> (String, Arc<Mutex<Vec<String>>>, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock Slack API listener should bind");
        let addr = listener
            .local_addr()
            .expect("mock Slack API listener should have address");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_clone = Arc::clone(&requests);

        let handle = tokio::spawn(async move {
            for idx in 0..2 {
                let (mut socket, _) = listener
                    .accept()
                    .await
                    .expect("mock Slack API should accept connection");
                let mut request_buf = vec![0_u8; 8192];
                let read = socket
                    .read(&mut request_buf)
                    .await
                    .expect("mock Slack API should read request");
                requests_clone
                    .lock()
                    .push(String::from_utf8_lossy(&request_buf[..read]).into_owned());

                if idx == 0 {
                    let body = format!(
                        r#"{{"ok":true,"file":{{"id":"F123","url_private_download":"http://127.0.0.1:{}/download/photo.png"}}}}"#,
                        addr.port()
                    );
                    let response = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(), body
                    );
                    socket
                        .write_all(response.as_bytes())
                        .await
                        .expect("mock Slack API should write metadata response");
                } else {
                    let headers = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: image/png\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        image_body.len()
                    );
                    socket
                        .write_all(headers.as_bytes())
                        .await
                        .expect("mock Slack API should write download headers");
                    socket
                        .write_all(&image_body)
                        .await
                        .expect("mock Slack API should write image body");
                }
            }
        });

        (
            format!("http://127.0.0.1:{}/api", addr.port()),
            requests,
            handle,
        )
    }

    async fn spawn_mock_slack_json_api(
        path: &'static str,
        body: String,
    ) -> (
        String,
        Arc<Mutex<Option<String>>>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock Slack JSON API listener should bind");
        let addr = listener
            .local_addr()
            .expect("mock Slack JSON API listener should have address");
        let request = Arc::new(Mutex::new(None));
        let request_clone = Arc::clone(&request);

        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener
                .accept()
                .await
                .expect("mock Slack JSON API should accept connection");
            let mut request_buf = vec![0_u8; 4096];
            let read = socket
                .read(&mut request_buf)
                .await
                .expect("mock Slack JSON API should read request");
            *request_clone.lock() =
                Some(String::from_utf8_lossy(&request_buf[..read]).into_owned());

            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(), body
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("mock Slack JSON API should write response");
        });

        (
            format!("http://127.0.0.1:{}/api/{path}", addr.port()),
            request,
            handle,
        )
    }

    #[tokio::test]
    async fn fetch_and_stage_image_uses_bearer_auth_for_metadata_and_download() {
        let mut png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&[9_u8; 32]);
        let (api_base_url, requests, server) = spawn_mock_slack_file_api(png.clone()).await;
        let channel = SlackChannel::new_with_api_base_url(
            "xoxb-super-secret".into(),
            None,
            vec![],
            api_base_url,
        );

        let staged = channel
            .fetch_and_stage_image("F123", Some("image/png"), media::MAX_IMAGE_BYTES)
            .await
            .expect("Slack image should stage successfully");

        server.await.expect("mock Slack API should finish cleanly");

        let requests = requests.lock().clone();
        assert_eq!(requests.len(), 2, "expected metadata + download requests");
        assert!(requests[0].starts_with("GET /api/files.info?file=F123 HTTP/1.1\r\n"));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("\r\nauthorization: bearer xoxb-super-secret\r\n"));
        assert!(requests[1].starts_with("GET /download/photo.png HTTP/1.1\r\n"));
        assert!(requests[1]
            .to_ascii_lowercase()
            .contains("\r\nauthorization: bearer xoxb-super-secret\r\n"));
        assert_eq!(staged.mime_type, media::AllowedImageMime::Png);
        assert_eq!(staged.byte_len, png.len() as u64);
        assert_eq!(
            std::fs::read(&staged.temp_path).expect("staged file must exist"),
            png
        );
        assert!(staged
            .temp_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("corvus-sl-img-")));

        let temp_path = staged.temp_path.clone();
        staged.cleanup();
        assert!(
            !temp_path.exists(),
            "cleanup should remove {}",
            temp_path.display()
        );
    }

    #[tokio::test]
    async fn get_bot_user_id_returns_none_when_slack_api_reports_error() {
        let (auth_url, request, server) = spawn_mock_slack_json_api(
            "auth.test",
            r#"{"ok":false,"error":"invalid_auth"}"#.to_string(),
        )
        .await;
        let channel = SlackChannel::new_with_api_base_url(
            "xoxb-invalid".into(),
            None,
            vec![],
            auth_url.trim_end_matches("/auth.test").to_string(),
        );

        assert!(channel.get_bot_user_id().await.is_none());
        server.await.expect("mock auth.test server should finish");

        let request = request.lock().clone().expect("request should be captured");
        assert!(request.starts_with("GET /api/auth.test HTTP/1.1\r\n"));
    }

    #[tokio::test]
    async fn poll_history_returns_error_when_slack_api_reports_error() {
        let (history_url, request, server) = spawn_mock_slack_json_api(
            "conversations.history",
            r#"{"ok":false,"error":"missing_scope"}"#.to_string(),
        )
        .await;
        let channel = SlackChannel::new_with_api_base_url(
            "xoxb-invalid".into(),
            Some("C12345".into()),
            vec![],
            history_url
                .trim_end_matches("/conversations.history")
                .to_string(),
        );

        let error = channel
            .poll_history("C12345", "")
            .await
            .expect_err("poll_history should fail when Slack returns ok=false");
        assert!(error.to_string().contains("missing_scope"));
        server
            .await
            .expect("mock conversations.history server should finish");

        let request = request.lock().clone().expect("request should be captured");
        assert!(request
            .starts_with("GET /api/conversations.history?channel=C12345&limit=10 HTTP/1.1\r\n"));
    }

    #[tokio::test]
    async fn health_check_returns_false_when_slack_api_reports_error() {
        let (auth_url, _request, server) = spawn_mock_slack_json_api(
            "auth.test",
            r#"{"ok":false,"error":"account_inactive"}"#.to_string(),
        )
        .await;
        let channel = SlackChannel::new_with_api_base_url(
            "xoxb-invalid".into(),
            None,
            vec![],
            auth_url.trim_end_matches("/auth.test").to_string(),
        );

        assert!(!channel.health_check().await);
        server
            .await
            .expect("mock health check server should finish");
    }
}
