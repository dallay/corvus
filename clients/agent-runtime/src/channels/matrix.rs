use crate::channels::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::mpsc;

/// Matrix channel using the Client-Server API (no SDK needed).
/// Connects to any Matrix homeserver (Element, Synapse, etc.).
#[derive(Clone)]
pub struct MatrixChannel {
    homeserver: String,
    access_token: String,
    room_id: String,
    allowed_users: Vec<String>,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct SyncResponse {
    next_batch: String,
    #[serde(default)]
    rooms: Rooms,
}

#[derive(Debug, Deserialize, Default)]
struct Rooms {
    #[serde(default)]
    join: std::collections::HashMap<String, JoinedRoom>,
}

#[derive(Debug, Deserialize)]
struct JoinedRoom {
    #[serde(default)]
    timeline: Timeline,
}

#[derive(Debug, Deserialize, Default)]
struct Timeline {
    #[serde(default)]
    events: Vec<TimelineEvent>,
}

#[derive(Debug, Deserialize)]
struct TimelineEvent {
    #[serde(rename = "type")]
    event_type: String,
    sender: String,
    #[serde(default)]
    content: EventContent,
}

#[derive(Debug, Deserialize, Default)]
struct EventContent {
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    msgtype: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WhoAmIResponse {
    user_id: String,
}

impl MatrixChannel {
    pub fn new(
        homeserver: String,
        access_token: String,
        room_id: String,
        allowed_users: Vec<String>,
    ) -> Self {
        let homeserver = if homeserver.ends_with('/') {
            homeserver[..homeserver.len() - 1].to_string()
        } else {
            homeserver
        };
        Self {
            homeserver,
            access_token,
            room_id,
            allowed_users,
            client: Client::new(),
        }
    }

    fn is_user_allowed(&self, sender: &str) -> bool {
        if self.allowed_users.iter().any(|u| u == "*") {
            return true;
        }
        self.allowed_users
            .iter()
            .any(|u| u.eq_ignore_ascii_case(sender))
    }

    async fn get_my_user_id(&self) -> anyhow::Result<String> {
        let url = format!("{}/_matrix/client/v3/account/whoami", self.homeserver);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Matrix whoami failed: {err}");
        }

        let who: WhoAmIResponse = resp.json().await?;
        Ok(who.user_id)
    }

    /// Perform the initial Matrix sync to obtain the `since` token.
    async fn initial_sync(&self) -> anyhow::Result<String> {
        let url = format!(
            "{}/_matrix/client/v3/sync?timeout=30000&filter={{\"room\":{{\"timeline\":{{\"limit\":1}}}}}}",
            self.homeserver
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Matrix initial sync failed: {err}");
        }

        let sync: SyncResponse = resp.json().await?;
        Ok(sync.next_batch)
    }

    /// Long-poll for the next sync batch. Returns an error on network failure
    /// or non-success status so the caller can retry.
    async fn poll_sync(&self, since: &str) -> anyhow::Result<SyncResponse> {
        let url = format!(
            "{}/_matrix/client/v3/sync?since={}&timeout=30000",
            self.homeserver, since
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| {
                tracing::warn!("Matrix sync error: {e}, retrying...");
                e
            })?;

        if !resp.status().is_success() {
            anyhow::bail!("Matrix sync returned non-success status");
        }

        Ok(resp.json().await?)
    }

    /// Process room events from a sync response, forwarding messages to the
    /// channel sender. Returns `true` when the sender is closed (listener
    /// should stop).
    async fn process_room_events(
        &self,
        sync: &SyncResponse,
        my_user_id: &str,
        tx: &mpsc::Sender<ChannelMessage>,
    ) -> bool {
        let Some(room) = sync.rooms.join.get(&self.room_id) else {
            return false;
        };
        for event in &room.timeline.events {
            if let Some(msg) = self.event_to_message(event, my_user_id) {
                if tx.send(msg).await.is_err() {
                    return true;
                }
            }
        }
        false
    }

    /// Convert a single timeline event into a `ChannelMessage`, filtering out
    /// own messages, non-text types, and unauthorized senders.
    fn event_to_message(&self, event: &TimelineEvent, my_user_id: &str) -> Option<ChannelMessage> {
        if event.sender == my_user_id {
            return None;
        }
        if event.event_type != "m.room.message" {
            return None;
        }
        if event.content.msgtype.as_deref() != Some("m.text") {
            return None;
        }
        let body = event.content.body.as_ref()?;
        if !self.is_user_allowed(&event.sender) {
            return None;
        }

        Some(ChannelMessage {
            id: format!("mx_{}", chrono::Utc::now().timestamp_millis()),
            sender: event.sender.clone(),
            reply_target: event.sender.clone(),
            content: body.clone(),
            channel: "matrix".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            parts: vec![],
        })
    }
}

#[async_trait]
impl Channel for MatrixChannel {
    fn name(&self) -> &str {
        "matrix"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        let txn_id = format!("zc_{}", chrono::Utc::now().timestamp_millis());
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            self.homeserver, self.room_id, txn_id
        );

        let body = serde_json::json!({
            "msgtype": "m.text",
            "body": message.content
        });

        let resp = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Matrix send failed: {err}");
        }

        Ok(())
    }

    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        tracing::info!("Matrix channel listening on room {}...", self.room_id);

        let my_user_id = self.get_my_user_id().await?;
        let mut since = self.initial_sync().await?;

        // Long-poll loop
        loop {
            let sync = match self.poll_sync(&since).await {
                Ok(s) => s,
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
            };
            since = sync.next_batch.clone();

            if self.process_room_events(&sync, &my_user_id, &tx).await {
                return Ok(());
            }
        }
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/_matrix/client/v3/account/whoami", self.homeserver);
        let Ok(resp) = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
        else {
            return false;
        };

        resp.status().is_success()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_channel() -> MatrixChannel {
        MatrixChannel::new(
            "https://matrix.org".to_string(),
            "syt_test_token".to_string(),
            "!room:matrix.org".to_string(),
            vec!["@user:matrix.org".to_string()],
        )
    }

    #[test]
    fn creates_with_correct_fields() {
        let ch = make_channel();
        assert_eq!(ch.homeserver, "https://matrix.org");
        assert_eq!(ch.access_token, "syt_test_token");
        assert_eq!(ch.room_id, "!room:matrix.org");
        assert_eq!(ch.allowed_users.len(), 1);
    }

    #[test]
    fn strips_trailing_slash() {
        let ch = MatrixChannel::new(
            "https://matrix.org/".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
        );
        assert_eq!(ch.homeserver, "https://matrix.org");
    }

    #[test]
    fn no_trailing_slash_unchanged() {
        let ch = MatrixChannel::new(
            "https://matrix.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
        );
        assert_eq!(ch.homeserver, "https://matrix.org");
    }

    #[test]
    fn multiple_trailing_slashes_strips_one() {
        let ch = MatrixChannel::new(
            "https://matrix.org//".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
        );
        assert_eq!(ch.homeserver, "https://matrix.org/");
    }

    #[test]
    fn wildcard_allows_anyone() {
        let ch = MatrixChannel::new(
            "https://m.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec!["*".to_string()],
        );
        assert!(ch.is_user_allowed("@anyone:matrix.org"));
        assert!(ch.is_user_allowed("@hacker:evil.org"));
    }

    #[test]
    fn specific_user_allowed() {
        let ch = make_channel();
        assert!(ch.is_user_allowed("@user:matrix.org"));
    }

    #[test]
    fn unknown_user_denied() {
        let ch = make_channel();
        assert!(!ch.is_user_allowed("@stranger:matrix.org"));
        assert!(!ch.is_user_allowed("@evil:hacker.org"));
    }

    #[test]
    fn user_case_insensitive() {
        let ch = MatrixChannel::new(
            "https://m.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec!["@User:Matrix.org".to_string()],
        );
        assert!(ch.is_user_allowed("@user:matrix.org"));
        assert!(ch.is_user_allowed("@USER:MATRIX.ORG"));
    }

    #[test]
    fn empty_allowlist_denies_all() {
        let ch = MatrixChannel::new(
            "https://m.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
        );
        assert!(!ch.is_user_allowed("@anyone:matrix.org"));
    }

    #[test]
    fn name_returns_matrix() {
        let ch = make_channel();
        assert_eq!(ch.name(), "matrix");
    }

    #[test]
    fn sync_response_deserializes_empty() {
        let json = r#"{"next_batch":"s123","rooms":{"join":{}}}"#;
        let resp: SyncResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.next_batch, "s123");
        assert!(resp.rooms.join.is_empty());
    }

    #[test]
    fn sync_response_deserializes_with_events() {
        let json = r#"{
            "next_batch": "s456",
            "rooms": {
                "join": {
                    "!room:matrix.org": {
                        "timeline": {
                            "events": [
                                {
                                    "type": "m.room.message",
                                    "sender": "@user:matrix.org",
                                    "content": {
                                        "msgtype": "m.text",
                                        "body": "Hello!"
                                    }
                                }
                            ]
                        }
                    }
                }
            }
        }"#;
        let resp: SyncResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.next_batch, "s456");
        let room = resp.rooms.join.get("!room:matrix.org").unwrap();
        assert_eq!(room.timeline.events.len(), 1);
        assert_eq!(room.timeline.events[0].sender, "@user:matrix.org");
        assert_eq!(
            room.timeline.events[0].content.body.as_deref(),
            Some("Hello!")
        );
        assert_eq!(
            room.timeline.events[0].content.msgtype.as_deref(),
            Some("m.text")
        );
    }

    #[test]
    fn sync_response_ignores_non_text_events() {
        let json = r#"{
            "next_batch": "s789",
            "rooms": {
                "join": {
                    "!room:m": {
                        "timeline": {
                            "events": [
                                {
                                    "type": "m.room.member",
                                    "sender": "@user:m",
                                    "content": {}
                                }
                            ]
                        }
                    }
                }
            }
        }"#;
        let resp: SyncResponse = serde_json::from_str(json).unwrap();
        let room = resp.rooms.join.get("!room:m").unwrap();
        assert_eq!(room.timeline.events[0].event_type, "m.room.member");
        assert!(room.timeline.events[0].content.body.is_none());
    }

    #[test]
    fn whoami_response_deserializes() {
        let json = r#"{"user_id":"@bot:matrix.org"}"#;
        let resp: WhoAmIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.user_id, "@bot:matrix.org");
    }

    #[test]
    fn event_content_defaults() {
        let json = r#"{"type":"m.room.message","sender":"@u:m","content":{}}"#;
        let event: TimelineEvent = serde_json::from_str(json).unwrap();
        assert!(event.content.body.is_none());
        assert!(event.content.msgtype.is_none());
    }

    #[test]
    fn sync_response_missing_rooms_defaults() {
        let json = r#"{"next_batch":"s0"}"#;
        let resp: SyncResponse = serde_json::from_str(json).unwrap();
        assert!(resp.rooms.join.is_empty());
    }

    // ── event_to_message ─────────────────────────────────────────

    fn make_event(
        event_type: &str,
        sender: &str,
        msgtype: Option<&str>,
        body: Option<&str>,
    ) -> TimelineEvent {
        TimelineEvent {
            event_type: event_type.to_string(),
            sender: sender.to_string(),
            content: EventContent {
                msgtype: msgtype.map(String::from),
                body: body.map(String::from),
            },
        }
    }

    #[test]
    fn event_to_message_valid_text() {
        let ch = make_channel();
        let event = make_event(
            "m.room.message",
            "@user:matrix.org",
            Some("m.text"),
            Some("Hello!"),
        );
        let msg = ch.event_to_message(&event, "@bot:matrix.org");
        assert!(msg.is_some());
        let msg = msg.unwrap();
        assert_eq!(msg.sender, "@user:matrix.org");
        assert_eq!(msg.content, "Hello!");
        assert_eq!(msg.channel, "matrix");
    }

    #[test]
    fn event_to_message_skips_own_messages() {
        let ch = make_channel();
        let event = make_event(
            "m.room.message",
            "@bot:matrix.org",
            Some("m.text"),
            Some("echo"),
        );
        assert!(ch.event_to_message(&event, "@bot:matrix.org").is_none());
    }

    #[test]
    fn event_to_message_skips_non_room_message() {
        let ch = make_channel();
        let event = make_event("m.room.member", "@user:matrix.org", None, None);
        assert!(ch.event_to_message(&event, "@bot:matrix.org").is_none());
    }

    #[test]
    fn event_to_message_skips_non_text_msgtype() {
        let ch = make_channel();
        let event = make_event(
            "m.room.message",
            "@user:matrix.org",
            Some("m.image"),
            Some("pic.jpg"),
        );
        assert!(ch.event_to_message(&event, "@bot:matrix.org").is_none());
    }

    #[test]
    fn event_to_message_skips_unauthorized_sender() {
        let ch = make_channel();
        let event = make_event(
            "m.room.message",
            "@stranger:evil.org",
            Some("m.text"),
            Some("Hi"),
        );
        assert!(ch.event_to_message(&event, "@bot:matrix.org").is_none());
    }

    #[test]
    fn event_to_message_skips_missing_body() {
        let ch = make_channel();
        let event = make_event("m.room.message", "@user:matrix.org", Some("m.text"), None);
        assert!(ch.event_to_message(&event, "@bot:matrix.org").is_none());
    }
}
