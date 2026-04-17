use crate::multimedia::{AudioHistoryMeta, ImageHistoryMeta, ImageTransportForm, StagedImage};
use crate::tools::ToolSpec;
use async_trait::async_trait;
use futures_util::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    /// Image metadata for history turns (None for text-only or non-history messages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_metadata: Option<Vec<ImageHistoryMeta>>,
    /// Audio metadata for history turns (None for non-audio messages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_metadata: Option<Vec<AudioHistoryMeta>>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
            image_metadata: None,
            audio_metadata: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
            image_metadata: None,
            audio_metadata: None,
        }
    }

    pub fn user_with_images(content: impl Into<String>, metadata: Vec<ImageHistoryMeta>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
            image_metadata: if metadata.is_empty() {
                None
            } else {
                Some(metadata)
            },
            audio_metadata: None,
        }
    }

    pub fn user_with_audio(content: impl Into<String>, metadata: Vec<AudioHistoryMeta>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
            image_metadata: None,
            audio_metadata: if metadata.is_empty() {
                None
            } else {
                Some(metadata)
            },
        }
    }

    /// Build a user turn carrying both image and audio metadata.
    pub fn user_with_media(
        content: impl Into<String>,
        image_metadata: Vec<ImageHistoryMeta>,
        audio_metadata: Vec<AudioHistoryMeta>,
    ) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
            image_metadata: if image_metadata.is_empty() {
                None
            } else {
                Some(image_metadata)
            },
            audio_metadata: if audio_metadata.is_empty() {
                None
            } else {
                Some(audio_metadata)
            },
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
            image_metadata: None,
            audio_metadata: None,
        }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: content.into(),
            image_metadata: None,
            audio_metadata: None,
        }
    }
}

/// A tool call requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// An LLM response that may contain text, tool calls, or both.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// Text content of the response (may be empty if only tool calls).
    pub text: Option<String>,
    /// Tool calls requested by the LLM.
    pub tool_calls: Vec<ToolCall>,
}

impl ChatResponse {
    /// True when the LLM wants to invoke at least one tool.
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }

    /// Convenience: return text content or empty string.
    pub fn text_or_empty(&self) -> &str {
        self.text.as_deref().unwrap_or("")
    }
}

/// Request payload for provider chat calls.
#[derive(Debug, Clone, Copy)]
pub struct ChatRequest<'a> {
    pub messages: &'a [ChatMessage],
    pub tools: Option<&'a [ToolSpec]>,
    /// Staged images for the current turn. Empty for text-only turns.
    /// Adapters read bytes from each `StagedImage.temp_path`.
    pub images: &'a [StagedImage],
}

/// A tool result to feed back to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub content: String,
}

/// A message in a multi-turn conversation, including tool interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ConversationMessage {
    /// Regular chat message (system, user, assistant).
    Chat(ChatMessage),
    /// Tool calls from the assistant (stored for history fidelity).
    AssistantToolCalls {
        text: Option<String>,
        tool_calls: Vec<ToolCall>,
    },
    /// Results of tool executions, fed back to the LLM.
    ToolResults(Vec<ToolResultMessage>),
}

/// A chunk of content from a streaming response.
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// Text delta for this chunk.
    pub delta: String,
    /// Whether this is the final chunk.
    pub is_final: bool,
    /// Approximate token count for this chunk (estimated).
    pub token_count: usize,
}

impl StreamChunk {
    /// Create a new non-final chunk.
    pub fn delta(text: impl Into<String>) -> Self {
        Self {
            delta: text.into(),
            is_final: false,
            token_count: 0,
        }
    }

    /// Create a final chunk.
    pub fn final_chunk() -> Self {
        Self {
            delta: String::new(),
            is_final: true,
            token_count: 0,
        }
    }

    /// Create an error chunk.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            delta: message.into(),
            is_final: true,
            token_count: 0,
        }
    }

    /// Estimate tokens (rough approximation: ~4 chars per token).
    pub fn with_token_estimate(mut self) -> Self {
        self.token_count = self.delta.len().div_ceil(4);
        self
    }
}

/// Options for streaming chat requests.
#[derive(Debug, Clone, Copy, Default)]
pub struct StreamOptions {
    /// Whether to enable streaming (default: true).
    pub enabled: bool,
    /// Whether to include token counts in chunks.
    pub count_tokens: bool,
}

impl StreamOptions {
    /// Create new streaming options with enabled flag.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            count_tokens: false,
        }
    }

    /// Enable token counting.
    pub fn with_token_count(mut self) -> Self {
        self.count_tokens = true;
        self
    }
}

/// Result type for streaming operations.
pub type StreamResult<T> = std::result::Result<T, StreamError>;

/// Errors that can occur during streaming.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("HTTP error: {0}")]
    Http(reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(serde_json::Error),

    #[error("Invalid SSE format: {0}")]
    InvalidSse(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Provider capabilities declaration.
///
/// Describes what features a provider supports, enabling intelligent
/// adaptation of tool calling modes and request formatting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderCapabilities {
    /// Whether the provider supports native tool calling via API primitives.
    ///
    /// When `true`, the provider can convert tool definitions to API-native
    /// formats (e.g., Gemini's functionDeclarations, Anthropic's input_schema).
    ///
    /// When `false`, tools must be injected via system prompt as text.
    pub native_tool_calling: bool,

    /// Whether the provider/model accepts inbound image parts.
    ///
    /// When `false` (default), the provider is treated as text-only
    /// for routing purposes and image turns must not be sent to it.
    pub image_input: bool,

    /// Which canonical image transport forms the adapter accepts.
    ///
    /// Empty means no image transport is supported. For MVP, only
    /// `ImageTransportForm::InlineBytes` is used.
    pub image_transport_forms: Vec<ImageTransportForm>,
}

impl ProviderCapabilities {
    /// Returns `true` only when the provider declares image support
    /// AND accepts at least one transport form.
    pub fn supports_image_input(&self) -> bool {
        self.image_input && !self.image_transport_forms.is_empty()
    }
}

/// Provider-specific tool payload formats.
///
/// Different LLM providers require different formats for tool definitions.
/// This enum encapsulates those variations, enabling providers to convert
/// from the unified `ToolSpec` format to their native API requirements.
#[derive(Debug, Clone)]
pub enum ToolsPayload {
    /// Gemini API format (functionDeclarations).
    Gemini {
        function_declarations: Vec<serde_json::Value>,
    },
    /// Anthropic Messages API format (tools with input_schema).
    Anthropic { tools: Vec<serde_json::Value> },
    /// OpenAI Chat Completions API format (tools with function).
    OpenAI { tools: Vec<serde_json::Value> },
    /// Prompt-guided fallback (tools injected as text in system prompt).
    PromptGuided { instructions: String },
}

/// Build a modified message list with tool instructions injected into
/// the system message. If no system message exists, one is prepended.
fn build_tool_augmented_messages(
    messages: &[ChatMessage],
    tool_instructions: &str,
) -> Vec<ChatMessage> {
    let mut modified = messages.to_vec();
    if let Some(sys) = modified.iter_mut().find(|m| m.role == "system") {
        if !sys.content.is_empty() {
            sys.content.push_str("\n\n");
        }
        sys.content.push_str(tool_instructions);
    } else {
        modified.insert(0, ChatMessage::system(tool_instructions.to_string()));
    }
    modified
}

#[async_trait]
pub trait Provider: Send + Sync {
    /// Query provider capabilities.
    ///
    /// Default implementation returns minimal capabilities (no native tool calling).
    /// Providers should override this to declare their actual capabilities.
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }

    /// Convert tool specifications to provider-native format.
    ///
    /// Default implementation returns `PromptGuided` payload, which injects
    /// tool documentation into the system prompt as text. Providers with
    /// native tool calling support should override this to return their
    /// specific format (Gemini, Anthropic, OpenAI).
    fn convert_tools(&self, tools: &[ToolSpec]) -> ToolsPayload {
        ToolsPayload::PromptGuided {
            instructions: build_tool_instructions_text(tools),
        }
    }

    /// Simple one-shot chat (single user message, no explicit system prompt).
    ///
    /// This is the preferred API for non-agentic direct interactions.
    async fn simple_chat(
        &self,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.chat_with_system(None, message, model, temperature)
            .await
    }

    /// One-shot chat with optional system prompt.
    ///
    /// Kept for compatibility and advanced one-shot prompting.
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String>;

    /// Multi-turn conversation. Default implementation extracts the last user
    /// message and delegates to `chat_with_system`.
    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let system = messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.as_str());
        let last_user = messages
            .iter()
            .rfind(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");
        self.chat_with_system(system, last_user, model, temperature)
            .await
    }

    /// Structured chat API for agent loop callers.
    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        if !request.images.is_empty() {
            anyhow::bail!("Provider does not support image input (no chat() override)");
        }

        if let Some(tools) = request.tools {
            if !tools.is_empty() && !self.supports_native_tools() {
                let tool_instructions = match self.convert_tools(tools) {
                    ToolsPayload::PromptGuided { instructions } => instructions,
                    payload => {
                        anyhow::bail!(
                            "Provider returned non-prompt-guided tools payload ({payload:?}) while supports_native_tools() is false"
                        )
                    }
                };
                let modified_messages =
                    build_tool_augmented_messages(request.messages, &tool_instructions);

                let text = self
                    .chat_with_history(&modified_messages, model, temperature)
                    .await?;
                return Ok(ChatResponse {
                    text: Some(text),
                    tool_calls: Vec::new(),
                });
            }
        }

        let text = self
            .chat_with_history(request.messages, model, temperature)
            .await?;
        Ok(ChatResponse {
            text: Some(text),
            tool_calls: Vec::new(),
        })
    }

    /// Whether provider supports native tool calls over API.
    fn supports_native_tools(&self) -> bool {
        self.capabilities().native_tool_calling
    }

    /// Warm up the HTTP connection pool (TLS handshake, DNS, HTTP/2 setup).
    /// Default implementation is a no-op; providers with HTTP clients should override.
    async fn warmup(&self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Chat with tool definitions for native function calling support.
    /// The default implementation falls back to chat_with_history and returns
    /// an empty tool_calls vector (prompt-based tool use only).
    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        _tools: &[serde_json::Value],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        let text = self.chat_with_history(messages, model, temperature).await?;
        Ok(ChatResponse {
            text: Some(text),
            tool_calls: Vec::new(),
        })
    }

    /// Whether provider supports streaming responses.
    /// Default implementation returns false.
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Streaming chat with optional system prompt.
    /// Returns an async stream of text chunks.
    /// Default implementation falls back to non-streaming chat.
    fn stream_chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
        _options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        stream::empty().boxed()
    }

    /// Streaming chat with history.
    /// Default implementation returns a single terminal error chunk.
    fn stream_chat_with_history(
        &self,
        _messages: &[ChatMessage],
        _model: &str,
        _temperature: f64,
        _options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        let chunk = StreamChunk::error("unknown does not support streaming");
        stream::once(async move { Ok(chunk) }).boxed()
    }
}

/// Build tool instructions text for prompt-guided tool calling.
///
/// Generates a formatted text block describing available tools and how to
/// invoke them using XML-style tags. This is used as a fallback when the
/// provider doesn't support native tool calling.
pub fn build_tool_instructions_text(tools: &[ToolSpec]) -> String {
    let mut instructions = String::new();

    instructions.push_str("## Tool Use Protocol\n\n");
    instructions.push_str("To use a tool, wrap a JSON object in <tool_call></tool_call> tags:\n\n");
    instructions.push_str("<tool_call>\n");
    instructions.push_str(r#"{"name": "tool_name", "arguments": {"param": "value"}}"#);
    instructions.push_str("\n</tool_call>\n\n");
    instructions.push_str("You may use multiple tool calls in a single response. ");
    instructions.push_str("After tool execution, results appear in <tool_result> tags. ");
    instructions
        .push_str("Continue reasoning with the results until you can give a final answer.\n\n");
    instructions.push_str("### Available Tools\n\n");

    for tool in tools {
        writeln!(&mut instructions, "**{}**: {}", tool.name, tool.description)
            .expect("writing to String cannot fail");

        let parameters =
            serde_json::to_string(&tool.parameters).unwrap_or_else(|_| "{}".to_string());
        writeln!(&mut instructions, "Parameters: `{parameters}`")
            .expect("writing to String cannot fail");
        instructions.push('\n');
    }

    instructions
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CapabilityMockProvider;

    struct EchoSystemProvider;

    #[async_trait]
    impl Provider for CapabilityMockProvider {
        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                native_tool_calling: true,
                ..Default::default()
            }
        }

        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok("ok".into())
        }
    }

    #[async_trait]
    impl Provider for EchoSystemProvider {
        async fn chat_with_system(
            &self,
            system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok(system_prompt.unwrap_or_default().to_string())
        }
    }

    #[test]
    fn chat_message_constructors() {
        let sys = ChatMessage::system("Be helpful");
        assert_eq!(sys.role, "system");
        assert_eq!(sys.content, "Be helpful");

        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, "user");

        let asst = ChatMessage::assistant("Hi there");
        assert_eq!(asst.role, "assistant");

        let tool = ChatMessage::tool("{}");
        assert_eq!(tool.role, "tool");
    }

    #[test]
    fn chat_response_helpers() {
        let empty = ChatResponse {
            text: None,
            tool_calls: vec![],
        };
        assert!(!empty.has_tool_calls());
        assert_eq!(empty.text_or_empty(), "");

        let with_tools = ChatResponse {
            text: Some("Let me check".into()),
            tool_calls: vec![ToolCall {
                id: "1".into(),
                name: "shell".into(),
                arguments: "{}".into(),
            }],
        };
        assert!(with_tools.has_tool_calls());
        assert_eq!(with_tools.text_or_empty(), "Let me check");
    }

    #[test]
    fn provider_capabilities_supports_image_input_requires_flag_and_transport() {
        let caps = ProviderCapabilities {
            image_input: true,
            image_transport_forms: vec![ImageTransportForm::InlineBytes],
            ..Default::default()
        };

        assert!(caps.supports_image_input());
        assert!(CapabilityMockProvider.supports_native_tools());
    }

    #[test]
    fn build_tool_instructions_text_lists_tools() {
        let instructions = build_tool_instructions_text(&[ToolSpec {
            name: "shell".into(),
            description: "Execute commands".into(),
            parameters: serde_json::json!({ "type": "object" }),
            source: None,
        }]);

        assert!(instructions.contains("Tool Use Protocol"));
        assert!(instructions.contains("**shell**"));
        assert!(instructions.contains("Execute commands"));
    }

    #[tokio::test]
    async fn provider_chat_injects_prompt_guided_tool_instructions() {
        let tools = [ToolSpec {
            name: "shell".into(),
            description: "Execute commands".into(),
            parameters: serde_json::json!({ "type": "object" }),
            source: None,
        }];

        let response = EchoSystemProvider
            .chat(
                ChatRequest {
                    messages: &[ChatMessage::system("BASE"), ChatMessage::user("hello")],
                    tools: Some(&tools),
                    images: &[],
                },
                "model",
                0.2,
            )
            .await
            .unwrap();

        let text = response.text.unwrap_or_default();
        assert!(text.contains("BASE"));
        assert!(text.contains("Tool Use Protocol"));
    }

    #[tokio::test]
    async fn provider_chat_rejects_images_without_override() {
        let image = StagedImage {
            sha256: "deadbeef".into(),
            mime_type: crate::multimedia::AllowedImageMime::Png,
            byte_len: 4,
            temp_path: std::path::PathBuf::from("/tmp/image.png"),
            transport_form: ImageTransportForm::InlineBytes,
            channel_origin: "test".into(),
        };

        let err = CapabilityMockProvider
            .chat(
                ChatRequest {
                    messages: &[ChatMessage::user("hello")],
                    tools: None,
                    images: &[image],
                },
                "model",
                0.2,
            )
            .await
            .unwrap_err();

        assert!(err.to_string().contains("does not support image input"));
    }

    #[test]
    fn chat_message_roundtrip_preserves_media_metadata() {
        let msg = ChatMessage::user_with_media(
            "Describe this",
            vec![ImageHistoryMeta {
                mime: "image/png".into(),
                sha256: "deadbeef".into(),
                byte_len: 2048,
                channel_origin: "discord".into(),
                caption: Some("caption".into()),
                description: Some("image".into()),
            }],
            vec![AudioHistoryMeta {
                mime: "audio/ogg".into(),
                sha256: "abc123".into(),
                byte_len: 4096,
                duration_secs: Some(10.0),
                channel_origin: "telegram".into(),
                transcription: "hello world".into(),
                caption: None,
            }],
        );

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.role, "user");
        assert_eq!(parsed.image_metadata.as_ref().map(Vec::len), Some(1));
        assert_eq!(parsed.audio_metadata.as_ref().map(Vec::len), Some(1));
    }
}
