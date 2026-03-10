use super::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const DINGTALK_BOT_CALLBACK_TOPIC: &str = "/v1.0/im/bot/messages/get";

/// DingTalk channel — connects via Stream Mode WebSocket for real-time messages.
/// Replies are sent through per-message session webhook URLs.
pub struct DingTalkChannel {
    client_id: String,
    client_secret: String,
    allowed_users: Vec<String>,
    client: reqwest::Client,
    /// Per-chat session webhooks for sending replies (chatID -> webhook URL).
    /// DingTalk provides a unique webhook URL with each incoming message.
    session_webhooks: Arc<RwLock<HashMap<String, String>>>,
}

/// Response from DingTalk gateway connection registration.
#[derive(serde::Deserialize)]
struct GatewayResponse {
    endpoint: String,
    ticket: String,
}

enum IncomingSocketFrame {
    Text(String),
    Continue,
    Break,
}

impl DingTalkChannel {
    pub fn new(client_id: String, client_secret: String, allowed_users: Vec<String>) -> Self {
        Self {
            client_id,
            client_secret,
            allowed_users,
            client: reqwest::Client::new(),
            session_webhooks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn is_user_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == user_id)
    }

    fn parse_stream_data(frame: &serde_json::Value) -> Option<serde_json::Value> {
        match frame.get("data") {
            Some(serde_json::Value::String(raw)) => serde_json::from_str(raw).ok(),
            Some(serde_json::Value::Object(_)) => frame.get("data").cloned(),
            _ => None,
        }
    }

    fn resolve_chat_id(data: &serde_json::Value, sender_id: &str) -> String {
        let is_private_chat = data
            .get("conversationType")
            .and_then(|value| {
                value
                    .as_str()
                    .map(|v| v == "1")
                    .or_else(|| value.as_i64().map(|v| v == 1))
            })
            .unwrap_or(true);

        if is_private_chat {
            sender_id.to_string()
        } else {
            data.get("conversationId")
                .and_then(|c| c.as_str())
                .unwrap_or(sender_id)
                .to_string()
        }
    }

    fn build_pong_response(message_id: &str, opaque: &str) -> String {
        serde_json::json!({
            "code": 200,
            "headers": {
                "contentType": "application/json",
                "messageId": message_id,
            },
            "message": "OK",
            "data": opaque,
        })
        .to_string()
    }

    fn build_ack_response(message_id: &str) -> String {
        serde_json::json!({
            "code": 200,
            "headers": {
                "contentType": "application/json",
                "messageId": message_id,
            },
            "message": "OK",
            "data": "",
        })
        .to_string()
    }

    fn extract_message_id(frame: &serde_json::Value) -> &str {
        frame
            .get("headers")
            .and_then(|h| h.get("messageId"))
            .and_then(|m| m.as_str())
            .unwrap_or("")
    }

    async fn handle_system_frame<S>(write: &mut S, frame: &serde_json::Value) -> bool
    where
        S: SinkExt<Message> + Unpin,
        <S as futures::Sink<Message>>::Error: std::fmt::Debug,
    {
        let message_id = Self::extract_message_id(frame);
        let opaque = frame
            .get("opaque")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let pong = Self::build_pong_response(message_id, opaque);
        match write.send(Message::Text(pong)).await {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!("DingTalk: failed to send pong: {:?}", e);
                false
            }
        }
    }

    async fn handle_event_callback(&self, frame: &serde_json::Value) -> Option<ChannelMessage> {
        let data = Self::parse_stream_data(frame)?;

        let content = data
            .get("text")
            .and_then(|t| t.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .trim();

        if content.is_empty() {
            return None;
        }

        let sender_id = data
            .get("senderStaffId")
            .and_then(|s| s.as_str())
            .unwrap_or("unknown");

        if !self.is_user_allowed(sender_id) {
            tracing::warn!("DingTalk: ignoring message from unauthorized user: {sender_id}");
            return None;
        }

        let chat_id = Self::resolve_chat_id(&data, sender_id);

        if let Some(webhook) = data.get("sessionWebhook").and_then(|w| w.as_str()) {
            let webhook = webhook.to_string();
            let mut webhooks = self.session_webhooks.write().await;
            webhooks.insert(chat_id.clone(), webhook.clone());
            webhooks.insert(sender_id.to_string(), webhook);
        }

        Some(ChannelMessage {
            id: Uuid::new_v4().to_string(),
            sender: sender_id.to_string(),
            reply_target: chat_id,
            content: content.to_string(),
            channel: "dingtalk".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
    }

    fn decode_socket_frame(
        message: Result<Message, tokio_tungstenite::tungstenite::Error>,
    ) -> IncomingSocketFrame {
        match message {
            Ok(Message::Text(text)) => IncomingSocketFrame::Text(text.to_string()),
            Ok(Message::Close(_)) => IncomingSocketFrame::Break,
            Err(error) => {
                tracing::warn!("DingTalk WebSocket error: {error}");
                IncomingSocketFrame::Break
            }
            _ => IncomingSocketFrame::Continue,
        }
    }

    async fn handle_stream_frame<S>(
        &self,
        write: &mut S,
        tx: &tokio::sync::mpsc::Sender<ChannelMessage>,
        frame: &serde_json::Value,
    ) -> bool
    where
        S: SinkExt<Message> + Unpin,
        <S as futures::Sink<Message>>::Error: std::fmt::Debug,
    {
        let frame_type = frame.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match frame_type {
            "SYSTEM" => match frame
                .get("headers")
                .and_then(|headers| headers.get("topic"))
                .and_then(|topic| topic.as_str())
            {
                Some("disconnect") => true,
                _ => Self::handle_system_frame(write, frame).await,
            },
            "EVENT" | "CALLBACK" => {
                let Some(channel_msg) = self.handle_event_callback(frame).await else {
                    return true;
                };

                if tx.send(channel_msg).await.is_err() {
                    tracing::warn!("DingTalk: message channel closed");
                    return false;
                }

                let message_id = Self::extract_message_id(frame);
                let ack = Self::build_ack_response(message_id);
                if let Err(error) = write.send(Message::Text(ack)).await {
                    tracing::warn!("DingTalk: failed to send ack: {:?}", error);
                    return false;
                }

                true
            }
            _ => true,
        }
    }

    /// Register a connection with DingTalk's gateway to get a WebSocket endpoint.
    async fn register_connection(&self) -> anyhow::Result<GatewayResponse> {
        let body = serde_json::json!({
            "clientId": self.client_id,
            "clientSecret": self.client_secret,
            "subscriptions": [
                {
                    "type": "CALLBACK",
                    "topic": DINGTALK_BOT_CALLBACK_TOPIC,
                }
            ],
        });

        let resp = self
            .client
            .post("https://api.dingtalk.com/v1.0/gateway/connections/open")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("DingTalk gateway registration failed ({status}): {err}");
        }

        let gw: GatewayResponse = resp.json().await?;
        Ok(gw)
    }
}

#[async_trait]
impl Channel for DingTalkChannel {
    fn name(&self) -> &str {
        "dingtalk"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        let webhooks = self.session_webhooks.read().await;
        let webhook_url = webhooks.get(&message.recipient).ok_or_else(|| {
            anyhow::anyhow!(
                "No session webhook found for chat {}. \
                 The user must send a message first to establish a session.",
                message.recipient
            )
        })?;

        let title = message.subject.as_deref().unwrap_or("Corvus");
        let body = serde_json::json!({
            "msgtype": "markdown",
            "markdown": {
                "title": title,
                "text": message.content,
            }
        });

        let resp = self.client.post(webhook_url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("DingTalk webhook reply failed ({status}): {err}");
        }

        Ok(())
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        tracing::info!("DingTalk: registering gateway connection...");

        let gw = self.register_connection().await?;
        let ws_url = format!("{}?ticket={}", gw.endpoint, gw.ticket);

        tracing::info!("DingTalk: connecting to stream WebSocket...");
        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        tracing::info!("DingTalk: connected and listening for messages...");

        while let Some(msg) = read.next().await {
            let msg = match Self::decode_socket_frame(msg) {
                IncomingSocketFrame::Text(text) => text,
                IncomingSocketFrame::Continue => continue,
                IncomingSocketFrame::Break => break,
            };

            let frame: serde_json::Value = match serde_json::from_str(&msg) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if !self.handle_stream_frame(&mut write, &tx, &frame).await {
                break;
            }
        }

        anyhow::bail!("DingTalk WebSocket stream ended")
    }

    async fn health_check(&self) -> bool {
        self.register_connection().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::Sink;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    #[derive(Default)]
    struct TestSink {
        messages: Vec<Message>,
        fail_on_send: bool,
    }

    impl Sink<Message> for TestSink {
        type Error = &'static str;

        fn poll_ready(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
            if self.fail_on_send {
                Err("send failed")
            } else {
                self.messages.push(item);
                Ok(())
            }
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    fn event_frame(frame_type: &str, content: &str) -> serde_json::Value {
        serde_json::json!({
            "type": frame_type,
            "headers": {
                "messageId": "msg-1",
            },
            "data": {
                "text": {
                    "content": content,
                },
                "senderStaffId": "staff-1",
                "conversationType": 1,
                "sessionWebhook": "https://example.com/hook"
            }
        })
    }

    #[test]
    fn test_name() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec![]);
        assert_eq!(ch.name(), "dingtalk");
    }

    #[test]
    fn test_user_allowed_wildcard() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        assert!(ch.is_user_allowed("anyone"));
    }

    #[test]
    fn test_user_allowed_specific() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec!["user123".into()]);
        assert!(ch.is_user_allowed("user123"));
        assert!(!ch.is_user_allowed("other"));
    }

    #[test]
    fn test_user_denied_empty() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec![]);
        assert!(!ch.is_user_allowed("anyone"));
    }

    #[test]
    fn test_config_serde() {
        let toml_str = r#"
client_id = "app_id_123"
client_secret = "secret_456"
allowed_users = ["user1", "*"]
"#;
        let config: crate::config::schema::DingTalkConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.client_id, "app_id_123");
        assert_eq!(config.client_secret, "secret_456");
        assert_eq!(config.allowed_users, vec!["user1", "*"]);
    }

    #[test]
    fn test_config_serde_defaults() {
        let toml_str = r#"
client_id = "id"
client_secret = "secret"
"#;
        let config: crate::config::schema::DingTalkConfig = toml::from_str(toml_str).unwrap();
        assert!(config.allowed_users.is_empty());
    }

    #[test]
    fn parse_stream_data_supports_string_payload() {
        let frame = serde_json::json!({
            "data": "{\"text\":{\"content\":\"hello\"}}"
        });
        let parsed = DingTalkChannel::parse_stream_data(&frame).unwrap();
        assert_eq!(
            parsed.get("text").and_then(|v| v.get("content")),
            Some(&serde_json::json!("hello"))
        );
    }

    #[test]
    fn parse_stream_data_supports_object_payload() {
        let frame = serde_json::json!({
            "data": {"text": {"content": "hello"}}
        });
        let parsed = DingTalkChannel::parse_stream_data(&frame).unwrap();
        assert_eq!(
            parsed.get("text").and_then(|v| v.get("content")),
            Some(&serde_json::json!("hello"))
        );
    }

    #[test]
    fn resolve_chat_id_handles_numeric_group_conversation_type() {
        let data = serde_json::json!({
            "conversationType": 2,
            "conversationId": "cid-group",
        });
        let chat_id = DingTalkChannel::resolve_chat_id(&data, "staff-1");
        assert_eq!(chat_id, "cid-group");
    }

    #[test]
    fn decode_socket_frame_handles_text() {
        let frame = DingTalkChannel::decode_socket_frame(Ok(Message::Text("hello".into())));
        assert!(matches!(frame, IncomingSocketFrame::Text(text) if text == "hello"));
    }

    #[test]
    fn decode_socket_frame_handles_close_errors_and_non_text_frames() {
        let close_frame = DingTalkChannel::decode_socket_frame(Ok(Message::Close(None)));
        assert!(matches!(close_frame, IncomingSocketFrame::Break));

        let error_frame = DingTalkChannel::decode_socket_frame(Err(
            tokio_tungstenite::tungstenite::Error::Io(std::io::Error::other("boom")),
        ));
        assert!(matches!(error_frame, IncomingSocketFrame::Break));

        let continue_frame = DingTalkChannel::decode_socket_frame(Ok(Message::Ping(Vec::new())));
        assert!(matches!(continue_frame, IncomingSocketFrame::Continue));
    }

    #[tokio::test]
    async fn handle_stream_frame_replies_to_system_ping() {
        let channel = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        let mut sink = TestSink::default();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let frame = serde_json::json!({
            "type": "SYSTEM",
            "headers": {
                "messageId": "system-1",
                "topic": "ping",
            }
            ,"opaque": "opaque-1"
        });

        assert!(channel.handle_stream_frame(&mut sink, &tx, &frame).await);
        assert_eq!(sink.messages.len(), 1);
        assert!(
            matches!(&sink.messages[0], Message::Text(text) if text.contains("system-1") && text.contains("opaque-1"))
        );
    }

    #[tokio::test]
    async fn handle_stream_frame_returns_false_when_system_pong_fails() {
        let channel = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        let mut sink = TestSink {
            fail_on_send: true,
            ..Default::default()
        };
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let frame = serde_json::json!({
            "type": "SYSTEM",
            "headers": {
                "messageId": "system-1",
                "topic": "ping",
            }
        });

        assert!(!channel.handle_stream_frame(&mut sink, &tx, &frame).await);
    }

    #[tokio::test]
    async fn handle_stream_frame_skips_disconnect_replies() {
        let channel = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        let mut sink = TestSink::default();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let frame = serde_json::json!({
            "type": "SYSTEM",
            "headers": {
                "messageId": "system-2",
                "topic": "disconnect",
            }
        });

        assert!(channel.handle_stream_frame(&mut sink, &tx, &frame).await);
        assert!(sink.messages.is_empty());
    }

    #[tokio::test]
    async fn handle_stream_frame_acks_and_forwards_event_messages() {
        let channel = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        let mut sink = TestSink::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        assert!(
            channel
                .handle_stream_frame(&mut sink, &tx, &event_frame("EVENT", "hello from dingtalk"))
                .await
        );

        let message = rx.recv().await.expect("message should be forwarded");
        assert_eq!(message.sender, "staff-1");
        assert_eq!(message.reply_target, "staff-1");
        assert_eq!(message.content, "hello from dingtalk");
        assert_eq!(sink.messages.len(), 1);
        assert!(matches!(&sink.messages[0], Message::Text(text) if text.contains("msg-1")));
    }

    #[tokio::test]
    async fn handle_stream_frame_ignores_empty_event_payloads() {
        let channel = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        let mut sink = TestSink::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        assert!(
            channel
                .handle_stream_frame(&mut sink, &tx, &event_frame("CALLBACK", "   "))
                .await
        );
        assert!(rx.try_recv().is_err());
        assert!(sink.messages.is_empty());
    }

    #[tokio::test]
    async fn handle_stream_frame_returns_false_when_channel_is_closed() {
        let channel = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        let mut sink = TestSink::default();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        drop(rx);

        assert!(
            !channel
                .handle_stream_frame(&mut sink, &tx, &event_frame("EVENT", "hello"))
                .await
        );
        assert!(sink.messages.is_empty());
    }

    #[tokio::test]
    async fn handle_stream_frame_returns_false_when_ack_send_fails() {
        let channel = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        let mut sink = TestSink {
            fail_on_send: true,
            ..Default::default()
        };
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        assert!(
            !channel
                .handle_stream_frame(&mut sink, &tx, &event_frame("CALLBACK", "hello"))
                .await
        );
        let message = rx
            .recv()
            .await
            .expect("message should be forwarded before ack");
        assert_eq!(message.content, "hello");
    }

    #[tokio::test]
    async fn handle_event_callback_ignores_unauthorized_users() {
        let channel = DingTalkChannel::new("id".into(), "secret".into(), vec!["staff-2".into()]);

        assert!(channel
            .handle_event_callback(&event_frame("EVENT", "hello from dingtalk"))
            .await
            .is_none());
    }

    #[tokio::test]
    async fn test_sink_close_is_ready() {
        let mut sink = TestSink::default();
        assert!(sink.close().await.is_ok());
    }
}
