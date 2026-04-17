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
    /// Join all text parts and media captions with double newlines.
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
                ContentPart::Image { caption_text, .. }
                | ContentPart::Audio { caption_text, .. } => caption_text
                    .as_deref()
                    .filter(|caption| !caption.is_empty()),
            })
            .collect();
        if blocks.is_empty() {
            self.content.clone()
        } else {
            blocks.join("\n\n")
        }
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

/// Message to send through a channel.
#[derive(Debug, Clone)]
pub struct SendMessage {
    pub content: String,
    pub recipient: String,
    pub subject: Option<String>,
}

impl SendMessage {
    /// Create a new message with content and recipient.
    pub fn new(content: impl Into<String>, recipient: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            recipient: recipient.into(),
            subject: None,
        }
    }

    /// Create a new message with content, recipient, and subject.
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

/// Core channel trait — implement for any messaging platform.
#[async_trait]
pub trait Channel: Send + Sync {
    /// Human-readable channel name.
    fn name(&self) -> &str;

    /// Send a message through this channel.
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()>;

    /// Start listening for incoming messages (long-running).
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()>;

    /// Check if channel is healthy.
    async fn health_check(&self) -> bool {
        true
    }

    /// Signal that the bot is processing a response (e.g. "typing" indicator).
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
    fn text_projection_prefers_parts_over_legacy_content() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: "legacy".into(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![
                ContentPart::Text {
                    text: "hello".into(),
                },
                ContentPart::Audio {
                    channel_handle: "audio-1".into(),
                    source_channel: "telegram".into(),
                    declared_mime: Some("audio/ogg".into()),
                    caption_text: Some("caption".into()),
                    file_name: None,
                    declared_bytes: None,
                    declared_duration_secs: Some(4),
                },
            ],
        };

        assert_eq!(msg.text_projection(), "hello\n\ncaption");
    }

    #[test]
    fn image_and_audio_helpers_filter_parts() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: String::new(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![
                ContentPart::Text {
                    text: "hello".into(),
                },
                ContentPart::Image {
                    channel_handle: "image-1".into(),
                    source_channel: "telegram".into(),
                    declared_mime: Some("image/png".into()),
                    caption_text: None,
                    file_name: None,
                    declared_bytes: None,
                },
                ContentPart::Audio {
                    channel_handle: "audio-1".into(),
                    source_channel: "telegram".into(),
                    declared_mime: Some("audio/ogg".into()),
                    caption_text: None,
                    file_name: None,
                    declared_bytes: None,
                    declared_duration_secs: Some(4),
                },
            ],
        };

        assert!(msg.has_image_parts());
        assert!(msg.has_audio_parts());
        assert_eq!(msg.image_parts().len(), 1);
        assert_eq!(msg.audio_parts().len(), 1);
    }

    #[test]
    fn text_projection_falls_back_to_legacy_content_when_parts_have_no_text() {
        let msg = ChannelMessage {
            id: "1".into(),
            sender: "alice".into(),
            reply_target: "alice".into(),
            content: "legacy fallback".into(),
            channel: "test".into(),
            timestamp: 0,
            parts: vec![
                ContentPart::Image {
                    channel_handle: "image-1".into(),
                    source_channel: "telegram".into(),
                    declared_mime: Some("image/png".into()),
                    caption_text: None,
                    file_name: None,
                    declared_bytes: None,
                },
                ContentPart::Audio {
                    channel_handle: "audio-1".into(),
                    source_channel: "telegram".into(),
                    declared_mime: Some("audio/ogg".into()),
                    caption_text: None,
                    file_name: None,
                    declared_bytes: None,
                    declared_duration_secs: Some(4),
                },
            ],
        };

        assert_eq!(msg.text_projection(), "legacy fallback");
    }

    #[tokio::test]
    async fn default_trait_methods_return_success() {
        let channel = DummyChannel;
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        assert!(channel.health_check().await);
        assert!(channel.start_typing("bob").await.is_ok());
        assert!(channel.stop_typing("bob").await.is_ok());
        assert!(!channel.supports_draft_updates());
        assert!(channel
            .send_draft(&SendMessage::new("draft", "bob"))
            .await
            .is_ok());
        assert!(channel.update_draft("bob", "msg_1", "text").await.is_ok());
        assert!(channel.finalize_draft("bob", "msg_1", "text").await.is_ok());

        assert!(channel
            .send(&SendMessage::new("hello", "bob"))
            .await
            .is_ok());
        assert!(channel.listen(tx).await.is_ok());

        let received = rx.recv().await;
        assert!(matches!(received, Some(ChannelMessage { sender, .. }) if sender == "tester"));
    }
}
