use async_trait::async_trait;

/// An ordered content part within a multimodal channel message.
#[derive(Debug, Clone)]
pub enum ContentPart {
    /// Plain text body or caption.
    Text { text: String },
    /// Image reference before fetch/staging.
    Image {
        channel_handle: String,
        source_channel: String,
        declared_mime: Option<String>,
        caption_text: Option<String>,
        file_name: Option<String>,
        declared_bytes: Option<u64>,
    },
    /// Audio reference before fetch/staging/transcription.
    Audio {
        channel_handle: String,
        source_channel: String,
        declared_mime: Option<String>,
        caption_text: Option<String>,
        file_name: Option<String>,
        declared_bytes: Option<u64>,
        /// Channel-reported duration in seconds (e.g., Telegram voice duration).
        declared_duration_secs: Option<u64>,
    },
}

/// A message received from or sent to a channel.
///
/// `content` is a legacy text-only projection kept for backward compatibility.
/// `parts` is the canonical source of truth for multimodal turns.
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    pub id: String,
    pub sender: String,
    pub reply_target: String,
    /// Legacy text projection — populated by channel parsers for compat.
    pub content: String,
    pub channel: String,
    pub timestamp: u64,
    /// Ordered multimodal parts (empty for text-only messages).
    pub parts: Vec<ContentPart>,
}

impl ChannelMessage {
    /// Join all text parts and image captions with double newlines.
    ///
    /// Caption text for image parts is emitted as a text block; no
    /// synthetic placeholders like `[image]` are inserted.
    pub fn text_projection(&self) -> String {
        if self.parts.is_empty() {
            return self.content.clone();
        }
        let blocks: Vec<&str> = self
            .parts
            .iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => {
                    if text.is_empty() {
                        None
                    } else {
                        Some(text.as_str())
                    }
                }
                ContentPart::Image { caption_text, .. } => {
                    caption_text.as_deref().filter(|c| !c.is_empty())
                }
                ContentPart::Audio { caption_text, .. } => {
                    caption_text.as_deref().filter(|c| !c.is_empty())
                }
            })
            .collect();
        blocks.join("\n\n")
    }

    /// Whether this message contains at least one image part.
    pub fn has_image_parts(&self) -> bool {
        self.parts
            .iter()
            .any(|p| matches!(p, ContentPart::Image { .. }))
    }

    /// Return only the image parts.
    pub fn image_parts(&self) -> Vec<&ContentPart> {
        self.parts
            .iter()
            .filter(|p| matches!(p, ContentPart::Image { .. }))
            .collect()
    }

    /// Whether this message contains at least one audio part.
    pub fn has_audio_parts(&self) -> bool {
        self.parts
            .iter()
            .any(|p| matches!(p, ContentPart::Audio { .. }))
    }

    /// Return only the audio parts.
    pub fn audio_parts(&self) -> Vec<&ContentPart> {
        self.parts
            .iter()
            .filter(|p| matches!(p, ContentPart::Audio { .. }))
            .collect()
    }
}

/// Message to send through a channel
#[derive(Debug, Clone)]
pub struct SendMessage {
    pub content: String,
    pub recipient: String,
    pub subject: Option<String>,
}

impl SendMessage {
    /// Create a new message with content and recipient
    pub fn new(content: impl Into<String>, recipient: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            recipient: recipient.into(),
            subject: None,
        }
    }

    /// Create a new message with content, recipient, and subject
    pub fn with_subject(
        content: impl Into<String>,
        recipient: impl Into<String>,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            recipient: recipient.into(),
            subject: Some(subject.into()),
        }
    }
}

/// Core channel trait — implement for any messaging platform
#[async_trait]
pub trait Channel: Send + Sync {
    /// Human-readable channel name
    fn name(&self) -> &str;

    /// Send a message through this channel
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()>;

    /// Start listening for incoming messages (long-running)
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()>;

    /// Check if channel is healthy
    async fn health_check(&self) -> bool {
        true
    }

    /// Signal that the bot is processing a response (e.g. "typing" indicator).
    /// Implementations should repeat the indicator as needed for their platform.
    async fn start_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        Ok(())
    }

    /// Stop any active typing indicator.
    async fn stop_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        Ok(())
    }

    /// Whether this channel supports progressive message updates via draft edits.
    fn supports_draft_updates(&self) -> bool {
        false
    }

    /// Send an initial draft message. Returns a platform-specific message ID for later edits.
    async fn send_draft(&self, _message: &SendMessage) -> anyhow::Result<Option<String>> {
        Ok(None)
    }

    /// Update a previously sent draft message with new accumulated content.
    async fn update_draft(
        &self,
        _recipient: &str,
        _message_id: &str,
        _text: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Finalize a draft with the complete response (e.g. apply Markdown formatting).
    async fn finalize_draft(
        &self,
        _recipient: &str,
        _message_id: &str,
        _text: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyChannel;

    #[async_trait]
    impl Channel for DummyChannel {
        fn name(&self) -> &str {
            "dummy"
        }

        async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
            Ok(())
        }

        async fn listen(
            &self,
            tx: tokio::sync::mpsc::Sender<ChannelMessage>,
        ) -> anyhow::Result<()> {
            tx.send(ChannelMessage {
                id: "1".into(),
                sender: "tester".into(),
                reply_target: "tester".into(),
                content: "hello".into(),
                channel: "dummy".into(),
                timestamp: 123,
                parts: vec![],
            })
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
        }
    }

    #[test]
    fn channel_message_clone_preserves_fields() {
        let message = ChannelMessage {
            id: "42".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: "ping".into(),
            channel: "dummy".into(),
            timestamp: 999,
            parts: vec![],
        };

        let cloned = message.clone();
        assert_eq!(cloned.id, "42");
        assert_eq!(cloned.sender, "alice");
        assert_eq!(cloned.reply_target, "alice");
        assert_eq!(cloned.content, "ping");
        assert_eq!(cloned.channel, "dummy");
        assert_eq!(cloned.timestamp, 999);
    }

    #[tokio::test]
    async fn default_trait_methods_return_success() {
        let channel = DummyChannel;

        assert!(channel.health_check().await);
        assert!(channel.start_typing("bob").await.is_ok());
        assert!(channel.stop_typing("bob").await.is_ok());
        assert!(channel
            .send(&SendMessage::new("hello", "bob"))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn default_draft_methods_return_success() {
        let channel = DummyChannel;

        assert!(!channel.supports_draft_updates());
        assert!(channel
            .send_draft(&SendMessage::new("draft", "bob"))
            .await
            .unwrap()
            .is_none());
        assert!(channel.update_draft("bob", "msg_1", "text").await.is_ok());
        assert!(channel
            .finalize_draft("bob", "msg_1", "final text")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn listen_sends_message_to_channel() {
        let channel = DummyChannel;
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        channel.listen(tx).await.unwrap();

        let received = rx.recv().await.expect("message should be sent");
        assert_eq!(received.sender, "tester");
        assert_eq!(received.content, "hello");
        assert_eq!(received.channel, "dummy");
    }

    // ── Multimodal projection tests (Task 1.5) ─────────────────

    #[test]
    fn text_projection_single_text_part() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![ContentPart::Text {
                text: "solo".into(),
            }],
        };
        assert_eq!(msg.text_projection(), "solo");
    }

    // ── Audio content part tests (Task 1.1 — audio-input-support) ──

    #[test]
    fn has_audio_parts_returns_false_for_text_only() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: "hello".into(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![ContentPart::Text {
                text: "hello".into(),
            }],
        };
        assert!(!msg.has_audio_parts());
    }

    #[test]
    fn has_audio_parts_returns_false_for_image_only() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![ContentPart::Image {
                channel_handle: "f".into(),
                source_channel: "tg".into(),
                declared_mime: None,
                caption_text: None,
                file_name: None,
                declared_bytes: None,
            }],
        };
        assert!(!msg.has_audio_parts());
    }

    #[test]
    fn has_audio_parts_returns_true_when_audio_present() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "telegram".into(),
            timestamp: 0,
            parts: vec![ContentPart::Audio {
                channel_handle: "file_abc".into(),
                source_channel: "telegram".into(),
                declared_mime: Some("audio/ogg".into()),
                caption_text: None,
                file_name: None,
                declared_bytes: Some(12345),
                declared_duration_secs: Some(5),
            }],
        };
        assert!(msg.has_audio_parts());
    }

    #[test]
    fn audio_parts_returns_only_audio() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "telegram".into(),
            timestamp: 0,
            parts: vec![
                ContentPart::Text {
                    text: "hello".into(),
                },
                ContentPart::Audio {
                    channel_handle: "file_abc".into(),
                    source_channel: "telegram".into(),
                    declared_mime: Some("audio/ogg".into()),
                    caption_text: None,
                    file_name: None,
                    declared_bytes: None,
                    declared_duration_secs: Some(10),
                },
                ContentPart::Image {
                    channel_handle: "img".into(),
                    source_channel: "tg".into(),
                    declared_mime: None,
                    caption_text: None,
                    file_name: None,
                    declared_bytes: None,
                },
            ],
        };
        let audio = msg.audio_parts();
        assert_eq!(audio.len(), 1);
        assert!(matches!(audio[0], ContentPart::Audio { .. }));
    }

    #[test]
    fn audio_parts_returns_empty_when_no_audio() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: "text".into(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![ContentPart::Text {
                text: "text".into(),
            }],
        };
        assert!(msg.audio_parts().is_empty());
    }

    #[test]
    fn text_projection_includes_audio_captions() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "telegram".into(),
            timestamp: 0,
            parts: vec![
                ContentPart::Text {
                    text: "translate this".into(),
                },
                ContentPart::Audio {
                    channel_handle: "file_abc".into(),
                    source_channel: "telegram".into(),
                    declared_mime: Some("audio/ogg".into()),
                    caption_text: Some("please translate".into()),
                    file_name: None,
                    declared_bytes: None,
                    declared_duration_secs: None,
                },
            ],
        };
        assert_eq!(msg.text_projection(), "translate this\n\nplease translate");
    }

    #[test]
    fn text_projection_skips_audio_without_caption() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "telegram".into(),
            timestamp: 0,
            parts: vec![ContentPart::Audio {
                channel_handle: "file_abc".into(),
                source_channel: "telegram".into(),
                declared_mime: Some("audio/ogg".into()),
                caption_text: None,
                file_name: None,
                declared_bytes: None,
                declared_duration_secs: Some(15),
            }],
        };
        assert_eq!(msg.text_projection(), "");
    }

    // ── Multimodal contract tests (Task 1.1) ──────────────────

    #[test]
    fn text_only_message_has_empty_parts() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: "hello".into(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![],
        };
        assert!(msg.parts.is_empty());
        assert!(!msg.has_image_parts());
        assert!(msg.image_parts().is_empty());
    }

    #[test]
    fn text_projection_joins_text_parts() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![
                ContentPart::Text {
                    text: "Hello".into(),
                },
                ContentPart::Text {
                    text: "World".into(),
                },
            ],
        };
        assert_eq!(msg.text_projection(), "Hello\n\nWorld");
    }

    #[test]
    fn text_projection_includes_image_captions() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![
                ContentPart::Text {
                    text: "Look at this".into(),
                },
                ContentPart::Image {
                    channel_handle: "file_123".into(),
                    source_channel: "telegram".into(),
                    declared_mime: Some("image/jpeg".into()),
                    caption_text: Some("A nice photo".into()),
                    file_name: None,
                    declared_bytes: None,
                },
            ],
        };
        assert_eq!(msg.text_projection(), "Look at this\n\nA nice photo");
    }

    #[test]
    fn text_projection_skips_empty_blocks() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![
                ContentPart::Text {
                    text: String::new(),
                },
                ContentPart::Text {
                    text: "Only this".into(),
                },
                ContentPart::Image {
                    channel_handle: "f".into(),
                    source_channel: "tg".into(),
                    declared_mime: None,
                    caption_text: None,
                    file_name: None,
                    declared_bytes: None,
                },
            ],
        };
        assert_eq!(msg.text_projection(), "Only this");
    }

    #[test]
    fn text_projection_empty_parts_returns_empty() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![],
        };
        assert_eq!(msg.text_projection(), "");
    }

    #[test]
    fn has_image_parts_detects_images() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![
                ContentPart::Text { text: "hi".into() },
                ContentPart::Image {
                    channel_handle: "f".into(),
                    source_channel: "tg".into(),
                    declared_mime: None,
                    caption_text: None,
                    file_name: None,
                    declared_bytes: None,
                },
            ],
        };
        assert!(msg.has_image_parts());
    }

    #[test]
    fn image_parts_returns_only_images() {
        let img = ContentPart::Image {
            channel_handle: "f".into(),
            source_channel: "tg".into(),
            declared_mime: None,
            caption_text: None,
            file_name: None,
            declared_bytes: None,
        };
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![ContentPart::Text { text: "hi".into() }, img.clone()],
        };
        let images = msg.image_parts();
        assert_eq!(images.len(), 1);
        assert!(matches!(images[0], ContentPart::Image { .. }));
    }
}
