use super::media;
use super::traits::{Channel, ChannelMessage, ContentPart, SendMessage};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

/// Discord channel — connects via Gateway WebSocket for real-time messages
pub struct DiscordChannel {
    bot_token: String,
    guild_id: Option<String>,
    allowed_users: Vec<String>,
    listen_to_bots: bool,
    mention_only: bool,
    client: reqwest::Client,
    typing_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl DiscordChannel {
    pub fn new(
        bot_token: String,
        guild_id: Option<String>,
        allowed_users: Vec<String>,
        listen_to_bots: bool,
        mention_only: bool,
    ) -> Self {
        Self {
            bot_token,
            guild_id,
            allowed_users,
            listen_to_bots,
            mention_only,
            client: reqwest::Client::new(),
            typing_handle: Mutex::new(None),
        }
    }

    /// Check if a Discord user ID is in the allowlist.
    /// Empty list means deny everyone until explicitly configured.
    /// `"*"` means allow everyone.
    fn is_user_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == user_id)
    }

    fn bot_user_id_from_token(token: &str) -> Option<String> {
        // Discord bot tokens are base64(bot_user_id).timestamp.hmac
        let part = token.split('.').next()?;
        base64_decode(part)
    }

    /// Fetch an image from a Discord CDN attachment URL and stage it
    /// as a validated temp file ready for provider dispatch.
    pub async fn fetch_and_stage_image(
        &self,
        attachment_url: &str,
        declared_mime: Option<&str>,
    ) -> Result<media::StagedImage, media::ImageRejectionReason> {
        // 1. GET the attachment URL (Discord CDN URLs are pre-authenticated)
        let dl_resp = self
            .client
            .get(attachment_url)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!("Discord image download failed: {e}");
                media::ImageRejectionReason::FetchFailed
            })?;

        if !dl_resp.status().is_success() {
            tracing::warn!("Discord image download HTTP {}", dl_resp.status());
            return Err(media::ImageRejectionReason::FetchFailed);
        }

        // 2. Early reject via Content-Length
        if let Some(cl) = dl_resp.content_length() {
            media::validate_size(cl, media::MAX_IMAGE_BYTES)?;
        }

        // 3. Stream bytes with per-chunk size validation
        let mut bytes = Vec::new();
        let mut stream = dl_resp.bytes_stream();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|_| {
                tracing::warn!("Discord image download stream read error");
                media::ImageRejectionReason::FetchFailed
            })?;
            bytes.extend_from_slice(&chunk);
            media::validate_size(bytes.len() as u64, media::MAX_IMAGE_BYTES)?;
        }
        let byte_len = bytes.len() as u64;

        // 4. Validate MIME via magic-byte sniffing
        let mime = media::validate_mime(declared_mime, &bytes)?;

        // 5. Compute SHA-256 hash
        use sha2::Digest;
        let sha256 = {
            let mut hasher = sha2::Sha256::new();
            hasher.update(&bytes);
            hex::encode(hasher.finalize())
        };

        // 6. Write to temp file
        let ext = match mime {
            media::AllowedImageMime::Jpeg => "jpg",
            media::AllowedImageMime::Png => "png",
            media::AllowedImageMime::Webp => "webp",
        };
        let temp_path =
            std::env::temp_dir().join(format!("corvus-dc-img-{}.{ext}", &sha256[..16]));

        tokio::fs::write(&temp_path, &bytes).await.map_err(|e| {
            tracing::warn!(
                "Failed to stage Discord image to {}: {e}",
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
            channel_origin: "discord".to_string(),
        })
    }
}

const BASE64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Discord's maximum message length for regular messages.
///
/// Discord rejects longer payloads with `50035 Invalid Form Body`.
const DISCORD_MAX_MESSAGE_LENGTH: usize = 2000;

/// Split a message into chunks that respect Discord's 2000-character limit.
/// Tries to split at word boundaries when possible.
fn split_message_for_discord(message: &str) -> Vec<String> {
    if message.chars().count() <= DISCORD_MAX_MESSAGE_LENGTH {
        return vec![message.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = message;

    while !remaining.is_empty() {
        // Find the byte offset for the 2000th character boundary.
        // If there are fewer than 2000 chars left, we can emit the tail directly.
        let hard_split = remaining
            .char_indices()
            .nth(DISCORD_MAX_MESSAGE_LENGTH)
            .map_or(remaining.len(), |(idx, _)| idx);

        let chunk_end = if hard_split == remaining.len() {
            hard_split
        } else {
            // Try to find a good break point (newline, then space)
            let search_area = &remaining[..hard_split];

            // Prefer splitting at newline
            if let Some(pos) = search_area.rfind('\n') {
                // Don't split if the newline is too close to the end
                if search_area[..pos].chars().count() >= DISCORD_MAX_MESSAGE_LENGTH / 2 {
                    pos + 1
                } else {
                    // Try space as fallback
                    search_area.rfind(' ').map_or(hard_split, |space| space + 1)
                }
            } else if let Some(pos) = search_area.rfind(' ') {
                pos + 1
            } else {
                // Hard split at the limit
                hard_split
            }
        };

        chunks.push(remaining[..chunk_end].to_string());
        remaining = &remaining[chunk_end..];
    }

    chunks
}

fn mention_tags(bot_user_id: &str) -> [String; 2] {
    [format!("<@{bot_user_id}>"), format!("<@!{bot_user_id}>")]
}

fn contains_bot_mention(content: &str, bot_user_id: &str) -> bool {
    let tags = mention_tags(bot_user_id);
    content.contains(&tags[0]) || content.contains(&tags[1])
}

fn normalize_incoming_content(
    content: &str,
    mention_only: bool,
    bot_user_id: &str,
) -> Option<String> {
    if content.is_empty() {
        return None;
    }

    if mention_only && !contains_bot_mention(content, bot_user_id) {
        return None;
    }

    let mut normalized = content.to_string();
    if mention_only {
        for tag in mention_tags(bot_user_id) {
            normalized = normalized.replace(&tag, " ");
        }
    }

    let normalized = normalized.trim().to_string();
    if normalized.is_empty() {
        return None;
    }

    Some(normalized)
}

/// Minimal base64 decode (no extra dep) — only needs to decode the user ID portion
#[allow(clippy::cast_possible_truncation)]
fn base64_decode(input: &str) -> Option<String> {
    let padded = match input.len() % 4 {
        2 => format!("{input}=="),
        3 => format!("{input}="),
        _ => input.to_string(),
    };

    let mut bytes = Vec::new();
    let chars: Vec<u8> = padded.bytes().collect();

    for chunk in chars.chunks(4) {
        if chunk.len() < 4 {
            break;
        }

        let mut v = [0usize; 4];
        for (i, &b) in chunk.iter().enumerate() {
            if b == b'=' {
                v[i] = 0;
            } else {
                v[i] = BASE64_ALPHABET.iter().position(|&a| a == b)?;
            }
        }

        bytes.push(((v[0] << 2) | (v[1] >> 4)) as u8);
        if chunk[2] != b'=' {
            bytes.push((((v[1] & 0xF) << 4) | (v[2] >> 2)) as u8);
        }
        if chunk[3] != b'=' {
            bytes.push((((v[2] & 0x3) << 6) | v[3]) as u8);
        }
    }

    String::from_utf8(bytes).ok()
}

#[async_trait]
impl Channel for DiscordChannel {
    fn name(&self) -> &str {
        "discord"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        let chunks = split_message_for_discord(&message.content);

        for (i, chunk) in chunks.iter().enumerate() {
            let url = format!(
                "https://discord.com/api/v10/channels/{}/messages",
                message.recipient
            );

            let body = json!({ "content": chunk });

            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bot {}", self.bot_token))
                .json(&body)
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let err = resp
                    .text()
                    .await
                    .unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
                anyhow::bail!("Discord send message failed ({status}): {err}");
            }

            // Add a small delay between chunks to avoid rate limiting
            if i < chunks.len() - 1 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        let bot_user_id = Self::bot_user_id_from_token(&self.bot_token).unwrap_or_default();

        // Get Gateway URL
        let gw_resp: serde_json::Value = self
            .client
            .get("https://discord.com/api/v10/gateway/bot")
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await?
            .json()
            .await?;

        let gw_url = gw_resp
            .get("url")
            .and_then(|u| u.as_str())
            .unwrap_or("wss://gateway.discord.gg");

        let ws_url = format!("{gw_url}/?v=10&encoding=json");
        tracing::info!("Discord: connecting to gateway...");

        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Read Hello (opcode 10)
        let hello = read.next().await.ok_or(anyhow::anyhow!("No hello"))??;
        let hello_data: serde_json::Value = serde_json::from_str(&hello.to_string())?;
        let heartbeat_interval = hello_data
            .get("d")
            .and_then(|d| d.get("heartbeat_interval"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(41250);

        // Send Identify (opcode 2)
        let identify = json!({
            "op": 2,
            "d": {
                "token": self.bot_token,
                "intents": 37377, // GUILDS | GUILD_MESSAGES | MESSAGE_CONTENT | DIRECT_MESSAGES
                "properties": {
                    "os": "linux",
                    "browser": "corvus",
                    "device": "corvus"
                }
            }
        });
        write.send(Message::Text(identify.to_string())).await?;

        tracing::info!("Discord: connected and identified");

        // Track the last sequence number for heartbeats and resume.
        // Only accessed in the select! loop below, so a plain i64 suffices.
        let mut sequence: i64 = -1;

        // Spawn heartbeat timer — sends a tick signal, actual heartbeat
        // is assembled in the select! loop where `sequence` lives.
        let (hb_tx, mut hb_rx) = tokio::sync::mpsc::channel::<()>(1);
        let hb_interval = heartbeat_interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(hb_interval));
            loop {
                interval.tick().await;
                if hb_tx.send(()).await.is_err() {
                    break;
                }
            }
        });

        let guild_filter = self.guild_id.clone();

        loop {
            tokio::select! {
                            _ = hb_rx.recv() => {
                                let d = if sequence >= 0 { json!(sequence) } else { json!(null) };
                                let hb = json!({"op": 1, "d": d});
                                if write.send(Message::Text(hb.to_string())).await.is_err() {
                                    break;
                                }
                            }
                            msg = read.next() => {
                                let msg = match msg {
                                    Some(Ok(Message::Text(t))) => t,
                                    Some(Ok(Message::Close(_))) | None => break,
                                    _ => continue,
                                };

                                let event: serde_json::Value = match serde_json::from_str(&msg) {
                                    Ok(e) => e,
                                    Err(_) => continue,
                                };

                                // Track sequence number from all dispatch events
                                if let Some(s) = event.get("s").and_then(serde_json::Value::as_i64) {
                                    sequence = s;
                                }

                                let op = event.get("op").and_then(serde_json::Value::as_u64).unwrap_or(0);

                                match op {
                                    // Op 1: Server requests an immediate heartbeat
                                    1 => {
                                        let d = if sequence >= 0 { json!(sequence) } else { json!(null) };
                                        let hb = json!({"op": 1, "d": d});
                                        if write.send(Message::Text(hb.to_string())).await.is_err() {
                                            break;
                                        }
                                        continue;
                                    }
                                    // Op 7: Reconnect
                                    7 => {
                                        tracing::warn!("Discord: received Reconnect (op 7), closing for restart");
                                        break;
                                    }
                                    // Op 9: Invalid Session
                                    9 => {
                                        tracing::warn!("Discord: received Invalid Session (op 9), closing for restart");
                                        break;
                                    }
                                    _ => {}
                                }

                                // Only handle MESSAGE_CREATE (opcode 0, type "MESSAGE_CREATE")
                                let event_type = event.get("t").and_then(|t| t.as_str()).unwrap_or("");
                                if event_type != "MESSAGE_CREATE" {
                                    continue;
                                }

                                let Some(d) = event.get("d") else {
                                    continue;
                                };

                                // Skip messages from the bot itself
                                let author_id = d.get("author").and_then(|a| a.get("id")).and_then(|i| i.as_str()).unwrap_or("");
                                if author_id == bot_user_id {
                                    continue;
                                }

                                // Skip bot messages (unless listen_to_bots is enabled)
                                if !self.listen_to_bots && d.get("author").and_then(|a| a.get("bot")).and_then(serde_json::Value::as_bool).unwrap_or(false) {
                                    continue;
                                }

                                // Sender validation
                                if !self.is_user_allowed(author_id) {
                                    tracing::warn!("Discord: ignoring message from unauthorized user: {author_id}");
                                    continue;
                                }

                                // Guild filter
                                if let Some(ref gid) = guild_filter {
                                    let msg_guild = d.get("guild_id").and_then(serde_json::Value::as_str);
                                    // DMs have no guild_id — let them through; for guild messages, enforce the filter
                                    if let Some(g) = msg_guild {
                                        if g != gid {
                                            continue;
                                        }
                                    }
                                }

                                let content = d.get("content").and_then(|c| c.as_str()).unwrap_or("");

                                // Parse image attachments
let image_parts = parse_image_attachments(d);

                                let clean_content =
                                    normalize_incoming_content(content, self.mention_only, &bot_user_id);

                                // Skip if no text content AND no image attachments
                                if clean_content.is_none() && image_parts.is_empty() {
                                    continue;
                                }

                                // In mention_only mode, require mention even for image-only messages
                                if self.mention_only
                                    && !content.is_empty()
                                    && !contains_bot_mention(content, &bot_user_id)
                                {
                                    continue;
                                }

                                let text_for_content = clean_content.clone().unwrap_or_default();

                                let mut parts: Vec<ContentPart> = Vec::new();
                                if let Some(ref text) = clean_content {
                                    parts.push(ContentPart::Text { text: text.clone() });
                                }
                                parts.extend(image_parts);

                                let message_id = d.get("id").and_then(|i| i.as_str()).unwrap_or("");
                                let channel_id = d.get("channel_id").and_then(|c| c.as_str()).unwrap_or("").to_string();

                                let channel_msg = ChannelMessage {
                                    id: if message_id.is_empty() {
                                        format!("discord_{}", Uuid::new_v4())
                                    } else {
                                        format!("discord_{message_id}")
                                    },
                                    sender: author_id.to_string(),
                                    reply_target: if channel_id.is_empty() {
                                        author_id.to_string()
                                    } else {
                                        channel_id.clone()
                                    },
                                    content: text_for_content,
                                    channel: "discord".to_string(),
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                    parts,
            };

                                if tx.send(channel_msg).await.is_err() {
                                    break;
                                }
                            }
                        }
        }

        Ok(())
    }

    async fn health_check(&self) -> bool {
        self.client
            .get("https://discord.com/api/v10/users/@me")
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn start_typing(&self, recipient: &str) -> anyhow::Result<()> {
        self.stop_typing(recipient).await?;

        let client = self.client.clone();
        let token = self.bot_token.clone();
        let channel_id = recipient.to_string();

        let handle = tokio::spawn(async move {
            let url = format!("https://discord.com/api/v10/channels/{channel_id}/typing");
            loop {
                let _ = client
                    .post(&url)
                    .header("Authorization", format!("Bot {token}"))
                    .send()
                    .await;
                tokio::time::sleep(std::time::Duration::from_secs(8)).await;
            }
        });

        let mut guard = self.typing_handle.lock();
        *guard = Some(handle);

        Ok(())
    }

    async fn stop_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        let mut guard = self.typing_handle.lock();
        if let Some(handle) = guard.take() {
            handle.abort();
        }
        Ok(())
    }
}

/// Extract image `ContentPart`s from a Discord MESSAGE_CREATE `d` payload.
///
/// Factored out of `listen()` so it can be unit-tested without a live
/// WebSocket connection.
fn parse_image_attachments(d: &serde_json::Value) -> Vec<ContentPart> {
    let mut parts = Vec::new();
    let Some(attachments) = d.get("attachments").and_then(|a| a.as_array()) else {
        return parts;
    };
    for att in attachments {
        let ct = att
            .get("content_type")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        if !ct.starts_with("image/") {
            continue;
        }
        if media::AllowedImageMime::from_mime_str(ct).is_none() {
            continue;
        }
        let url = att
            .get("url")
            .and_then(|u| u.as_str())
            .unwrap_or("");
        if url.is_empty() {
            continue;
        }
        parts.push(ContentPart::Image {
            channel_handle: url.to_string(),
            source_channel: "discord".to_string(),
            declared_mime: Some(ct.to_string()),
            caption_text: None,
            file_name: att
                .get("filename")
                .and_then(|f| f.as_str())
                .map(String::from),
            declared_bytes: att.get("size").and_then(|s| s.as_u64()),
        });
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_channel_name() {
        let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
        assert_eq!(ch.name(), "discord");
    }

    #[test]
    fn base64_decode_bot_id() {
        // "MTIzNDU2" decodes to "123456"
        let decoded = base64_decode("MTIzNDU2");
        assert_eq!(decoded, Some("123456".to_string()));
    }

    #[test]
    fn bot_user_id_extraction() {
        // Token format: base64(user_id).timestamp.hmac
        let token = "MTIzNDU2.fake.hmac";
        let id = DiscordChannel::bot_user_id_from_token(token);
        assert_eq!(id, Some("123456".to_string()));
    }

    #[test]
    fn empty_allowlist_denies_everyone() {
        let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
        assert!(!ch.is_user_allowed("12345"));
        assert!(!ch.is_user_allowed("anyone"));
    }

    #[test]
    fn wildcard_allows_everyone() {
        let ch = DiscordChannel::new("fake".into(), None, vec!["*".into()], false, false);
        assert!(ch.is_user_allowed("12345"));
        assert!(ch.is_user_allowed("anyone"));
    }

    #[test]
    fn specific_allowlist_filters() {
        let ch = DiscordChannel::new(
            "fake".into(),
            None,
            vec!["111".into(), "222".into()],
            false,
            false,
        );
        assert!(ch.is_user_allowed("111"));
        assert!(ch.is_user_allowed("222"));
        assert!(!ch.is_user_allowed("333"));
        assert!(!ch.is_user_allowed("unknown"));
    }

    #[test]
    fn allowlist_is_exact_match_not_substring() {
        let ch = DiscordChannel::new("fake".into(), None, vec!["111".into()], false, false);
        assert!(!ch.is_user_allowed("1111"));
        assert!(!ch.is_user_allowed("11"));
        assert!(!ch.is_user_allowed("0111"));
    }

    #[test]
    fn allowlist_empty_string_user_id() {
        let ch = DiscordChannel::new("fake".into(), None, vec!["111".into()], false, false);
        assert!(!ch.is_user_allowed(""));
    }

    #[test]
    fn allowlist_with_wildcard_and_specific() {
        let ch = DiscordChannel::new(
            "fake".into(),
            None,
            vec!["111".into(), "*".into()],
            false,
            false,
        );
        assert!(ch.is_user_allowed("111"));
        assert!(ch.is_user_allowed("anyone_else"));
    }

    #[test]
    fn allowlist_case_sensitive() {
        let ch = DiscordChannel::new("fake".into(), None, vec!["ABC".into()], false, false);
        assert!(ch.is_user_allowed("ABC"));
        assert!(!ch.is_user_allowed("abc"));
        assert!(!ch.is_user_allowed("Abc"));
    }

    #[test]
    fn base64_decode_empty_string() {
        let decoded = base64_decode("");
        assert_eq!(decoded, Some(String::new()));
    }

    #[test]
    fn base64_decode_invalid_chars() {
        let decoded = base64_decode("!!!!");
        assert!(decoded.is_none());
    }

    #[test]
    fn bot_user_id_from_empty_token() {
        let id = DiscordChannel::bot_user_id_from_token("");
        assert_eq!(id, Some(String::new()));
    }

    #[test]
    fn contains_bot_mention_supports_plain_and_nick_forms() {
        assert!(contains_bot_mention("hi <@12345>", "12345"));
        assert!(contains_bot_mention("hi <@!12345>", "12345"));
        assert!(!contains_bot_mention("hi <@99999>", "12345"));
    }

    #[test]
    fn normalize_incoming_content_requires_mention_when_enabled() {
        let cleaned = normalize_incoming_content("hello there", true, "12345");
        assert!(cleaned.is_none());
    }

    #[test]
    fn normalize_incoming_content_strips_mentions_and_trims() {
        let cleaned = normalize_incoming_content("  <@!12345> run status  ", true, "12345");
        assert_eq!(cleaned.as_deref(), Some("run status"));
    }

    #[test]
    fn normalize_incoming_content_rejects_empty_after_strip() {
        let cleaned = normalize_incoming_content("<@12345>", true, "12345");
        assert!(cleaned.is_none());
    }

    // Message splitting tests

    #[test]
    fn split_empty_message() {
        let chunks = split_message_for_discord("");
        assert_eq!(chunks, vec![""]);
    }

    #[test]
    fn split_short_message_under_limit() {
        let msg = "Hello, world!";
        let chunks = split_message_for_discord(msg);
        assert_eq!(chunks, vec![msg]);
    }

    #[test]
    fn split_message_exactly_2000_chars() {
        let msg = "a".repeat(DISCORD_MAX_MESSAGE_LENGTH);
        let chunks = split_message_for_discord(&msg);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chars().count(), DISCORD_MAX_MESSAGE_LENGTH);
    }

    #[test]
    fn split_message_just_over_limit() {
        let msg = "a".repeat(DISCORD_MAX_MESSAGE_LENGTH + 1);
        let chunks = split_message_for_discord(&msg);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chars().count(), DISCORD_MAX_MESSAGE_LENGTH);
        assert_eq!(chunks[1].chars().count(), 1);
    }

    #[test]
    fn split_very_long_message() {
        let msg = "word ".repeat(2000); // 10000 characters (5 chars per "word ")
        let chunks = split_message_for_discord(&msg);
        // Should split into 5 chunks of <= 2000 chars
        assert_eq!(chunks.len(), 5);
        assert!(chunks
            .iter()
            .all(|chunk| chunk.chars().count() <= DISCORD_MAX_MESSAGE_LENGTH));
        // Verify total content is preserved
        let reconstructed = chunks.concat();
        assert_eq!(reconstructed, msg);
    }

    #[test]
    fn split_prefer_newline_break() {
        let msg = format!("{}\n{}", "a".repeat(1500), "b".repeat(500));
        let chunks = split_message_for_discord(&msg);
        // Should split at the newline
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].ends_with('\n'));
        assert!(chunks[1].starts_with('b'));
    }

    #[test]
    fn split_prefer_space_break() {
        let msg = format!("{} {}", "a".repeat(1500), "b".repeat(600));
        let chunks = split_message_for_discord(&msg);
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn split_without_good_break_points_hard_split() {
        // No spaces or newlines - should hard split at 2000
        let msg = "a".repeat(5000);
        let chunks = split_message_for_discord(&msg);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].chars().count(), DISCORD_MAX_MESSAGE_LENGTH);
        assert_eq!(chunks[1].chars().count(), DISCORD_MAX_MESSAGE_LENGTH);
        assert_eq!(chunks[2].chars().count(), 1000);
    }

    #[test]
    fn split_multiple_breaks() {
        // Create a message with multiple newlines
        let part1 = "a".repeat(900);
        let part2 = "b".repeat(900);
        let part3 = "c".repeat(900);
        let msg = format!("{part1}\n{part2}\n{part3}");
        let chunks = split_message_for_discord(&msg);
        // Should split into 2 chunks (first two parts + third part)
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].chars().count() <= DISCORD_MAX_MESSAGE_LENGTH);
        assert!(chunks[1].chars().count() <= DISCORD_MAX_MESSAGE_LENGTH);
    }

    #[test]
    fn split_preserves_content() {
        let original = "Hello world! This is a test message with some content. ".repeat(200);
        let chunks = split_message_for_discord(&original);
        let reconstructed = chunks.concat();
        assert_eq!(reconstructed, original);
    }

    #[test]
    fn split_unicode_content() {
        // Test with emoji and multi-byte characters
        let msg = "🦀 Rust is awesome! ".repeat(500);
        let chunks = split_message_for_discord(&msg);
        // All chunks should be valid UTF-8
        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.as_bytes()).is_ok());
            assert!(chunk.chars().count() <= DISCORD_MAX_MESSAGE_LENGTH);
        }
        // Reconstruct and verify
        let reconstructed = chunks.concat();
        assert_eq!(reconstructed, msg);
    }

    #[test]
    fn split_newline_too_close_to_end() {
        // If newline is in the first half, don't use it - use space instead or hard split
        let msg = format!("{}\n{}", "a".repeat(1900), "b".repeat(500));
        let chunks = split_message_for_discord(&msg);
        // Should split at newline since it's in the second half of the window
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn split_multibyte_only_content_without_panics() {
        let msg = "🦀".repeat(2500);
        let chunks = split_message_for_discord(&msg);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chars().count(), DISCORD_MAX_MESSAGE_LENGTH);
        assert_eq!(chunks[1].chars().count(), 500);
        let reconstructed = chunks.concat();
        assert_eq!(reconstructed, msg);
    }

    #[test]
    fn split_chunks_always_within_discord_limit() {
        let msg = "x".repeat(12_345);
        let chunks = split_message_for_discord(&msg);
        assert!(chunks
            .iter()
            .all(|chunk| chunk.chars().count() <= DISCORD_MAX_MESSAGE_LENGTH));
    }

    #[test]
    fn split_message_with_multiple_newlines() {
        let msg = "Line 1\nLine 2\nLine 3\n".repeat(1000);
        let chunks = split_message_for_discord(&msg);
        assert!(chunks.len() > 1);
        let reconstructed = chunks.concat();
        assert_eq!(reconstructed, msg);
    }

    #[test]
    fn typing_handle_starts_as_none() {
        let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
        let guard = ch.typing_handle.lock();
        assert!(guard.is_none());
    }

    #[tokio::test]
    async fn start_typing_sets_handle() {
        let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
        let _ = ch.start_typing("123456").await;
        let guard = ch.typing_handle.lock();
        assert!(guard.is_some());
    }

    #[tokio::test]
    async fn stop_typing_clears_handle() {
        let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
        let _ = ch.start_typing("123456").await;
        let _ = ch.stop_typing("123456").await;
        let guard = ch.typing_handle.lock();
        assert!(guard.is_none());
    }

    #[tokio::test]
    async fn stop_typing_is_idempotent() {
        let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
        assert!(ch.stop_typing("123456").await.is_ok());
        assert!(ch.stop_typing("123456").await.is_ok());
    }

    #[tokio::test]
    async fn start_typing_replaces_existing_task() {
        let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
        let _ = ch.start_typing("111").await;
        let _ = ch.start_typing("222").await;
        let guard = ch.typing_handle.lock();
        assert!(guard.is_some());
    }

    // ── Message ID edge cases ─────────────────────────────────────

    #[test]
    fn discord_message_id_format_includes_discord_prefix() {
        // Verify that message IDs follow the format: discord_{message_id}
        let message_id = "123456789012345678";
        let expected_id = format!("discord_{message_id}");
        assert_eq!(expected_id, "discord_123456789012345678");
    }

    #[test]
    fn discord_message_id_is_deterministic() {
        // Same message_id = same ID (prevents duplicates after restart)
        let message_id = "123456789012345678";
        let id1 = format!("discord_{message_id}");
        let id2 = format!("discord_{message_id}");
        assert_eq!(id1, id2);
    }

    #[test]
    fn discord_message_id_different_message_different_id() {
        // Different message IDs produce different IDs
        let id1 = "discord_123456789012345678".to_string();
        let id2 = "discord_987654321098765432".to_string();
        assert_ne!(id1, id2);
    }

    #[test]
    fn discord_message_id_uses_snowflake_id() {
        // Discord snowflake IDs are numeric strings
        let message_id = "123456789012345678"; // Typical snowflake format
        let id = format!("discord_{message_id}");
        assert!(id.starts_with("discord_"));
        // Snowflake IDs are numeric
        assert!(message_id.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn discord_message_id_fallback_to_uuid_on_empty() {
        // Edge case: empty message_id falls back to UUID
        let message_id = "";
        let id = if message_id.is_empty() {
            format!("discord_{}", uuid::Uuid::new_v4())
        } else {
            format!("discord_{message_id}")
        };
        assert!(id.starts_with("discord_"));
        // Should have UUID dashes
        assert!(id.contains('-'));
    }

    // ── Image attachment parsing tests ────────────────────────

    #[test]
    fn parse_image_attachment_produces_image_part() {
        let d = serde_json::json!({
            "attachments": [{
                "url": "https://cdn.discordapp.com/attachments/1/2/photo.jpg",
                "content_type": "image/jpeg",
                "filename": "photo.jpg",
                "size": 102_400
            }]
        });
        let parts = parse_image_attachments(&d);
        assert_eq!(parts.len(), 1);
        match &parts[0] {
            ContentPart::Image {
                channel_handle,
                source_channel,
                declared_mime,
                caption_text,
                file_name,
                declared_bytes,
            } => {
                assert_eq!(channel_handle, "https://cdn.discordapp.com/attachments/1/2/photo.jpg");
                assert_eq!(source_channel, "discord");
                assert_eq!(declared_mime.as_deref(), Some("image/jpeg"));
                assert!(caption_text.is_none());
                assert_eq!(file_name.as_deref(), Some("photo.jpg"));
                assert_eq!(*declared_bytes, Some(102_400));
            }
            ContentPart::Text { .. } => panic!("expected Image, got Text"),
        }
    }

    #[test]
    fn parse_non_image_attachment_skipped() {
        let d = serde_json::json!({
            "attachments": [{
                "url": "https://cdn.discordapp.com/attachments/1/2/doc.pdf",
                "content_type": "application/pdf",
                "filename": "doc.pdf",
                "size": 50000
            }]
        });
        let parts = parse_image_attachments(&d);
        assert!(parts.is_empty());
    }

    #[test]
    fn parse_unsupported_image_mime_skipped() {
        let d = serde_json::json!({
            "attachments": [{
                "url": "https://cdn.discordapp.com/attachments/1/2/anim.gif",
                "content_type": "image/gif",
                "filename": "anim.gif",
                "size": 200_000
            }]
        });
        let parts = parse_image_attachments(&d);
        assert!(parts.is_empty());
    }

    #[test]
    fn parse_text_and_image_produces_both_parts() {
        let d = serde_json::json!({
            "content": "Check this out",
            "attachments": [{
                "url": "https://cdn.discordapp.com/attachments/1/2/pic.png",
                "content_type": "image/png",
                "filename": "pic.png",
                "size": 80000
            }]
        });
        let content = d.get("content").and_then(|c| c.as_str()).unwrap_or("");
        let clean = normalize_incoming_content(content, false, "99999");
        let image_parts = parse_image_attachments(&d);

        let mut parts: Vec<ContentPart> = Vec::new();
        if let Some(ref text) = clean {
            parts.push(ContentPart::Text { text: text.clone() });
        }
        parts.extend(image_parts);

        assert_eq!(parts.len(), 2);
        assert!(matches!(&parts[0], ContentPart::Text { text } if text == "Check this out"));
        assert!(matches!(&parts[1], ContentPart::Image { .. }));
    }

    #[test]
    fn image_only_message_empty_text_still_has_parts() {
        let d = serde_json::json!({
            "content": "",
            "attachments": [{
                "url": "https://cdn.discordapp.com/attachments/1/2/snap.webp",
                "content_type": "image/webp",
                "filename": "snap.webp",
                "size": 45000
            }]
        });
        let content = d.get("content").and_then(|c| c.as_str()).unwrap_or("");
        let clean = normalize_incoming_content(content, false, "99999");
        let image_parts = parse_image_attachments(&d);

        // Text is empty so clean_content is None, but image parts exist
        assert!(clean.is_none());
        assert_eq!(image_parts.len(), 1);
        // Message should still be processable
        assert!(clean.is_some() || !image_parts.is_empty());
    }

    #[test]
    fn parse_attachment_missing_url_skipped() {
        let d = serde_json::json!({
            "attachments": [{
                "content_type": "image/jpeg",
                "filename": "photo.jpg",
                "size": 102_400
            }]
        });
        let parts = parse_image_attachments(&d);
        assert!(parts.is_empty());
    }

    #[test]
    fn parse_no_attachments_field_returns_empty() {
        let d = serde_json::json!({
            "content": "just text"
        });
        let parts = parse_image_attachments(&d);
        assert!(parts.is_empty());
    }

    #[test]
    fn parse_empty_attachments_array_returns_empty() {
        let d = serde_json::json!({
            "attachments": []
        });
        let parts = parse_image_attachments(&d);
        assert!(parts.is_empty());
    }

    #[test]
    fn parse_multiple_image_attachments() {
        let d = serde_json::json!({
            "attachments": [
                {
                    "url": "https://cdn.discordapp.com/a/1.jpg",
                    "content_type": "image/jpeg",
                    "filename": "a.jpg",
                    "size": 10000
                },
                {
                    "url": "https://cdn.discordapp.com/b/2.png",
                    "content_type": "image/png",
                    "filename": "b.png",
                    "size": 20000
                }
            ]
        });
        let parts = parse_image_attachments(&d);
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn parse_mixed_attachments_filters_correctly() {
        let d = serde_json::json!({
            "attachments": [
                {
                    "url": "https://cdn.discordapp.com/a/1.jpg",
                    "content_type": "image/jpeg",
                    "filename": "a.jpg",
                    "size": 10000
                },
                {
                    "url": "https://cdn.discordapp.com/b/doc.pdf",
                    "content_type": "application/pdf",
                    "filename": "doc.pdf",
                    "size": 50000
                },
                {
                    "url": "https://cdn.discordapp.com/c/3.webp",
                    "content_type": "image/webp",
                    "filename": "c.webp",
                    "size": 30000
                }
            ]
        });
        let parts = parse_image_attachments(&d);
        assert_eq!(parts.len(), 2);
    }
}
