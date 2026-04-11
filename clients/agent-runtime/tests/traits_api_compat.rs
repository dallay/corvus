use std::any::TypeId;

use async_trait::async_trait;
use corvus::channels::audio_media::AudioHistoryMeta as RuntimeAudioHistoryMeta;
use corvus::channels::media::{
    AllowedImageMime as RuntimeAllowedImageMime, ImageHistoryMeta as RuntimeImageHistoryMeta,
    ImageTransportForm as RuntimeImageTransportForm, StagedImage as RuntimeStagedImage,
};
use corvus::channels::{self, audio_media, media, Channel as RuntimeChannel, SendMessage};
use corvus::memory::{self, Memory as RuntimeMemory, MemoryCategory};
use corvus::providers::traits::{
    ChatMessage as RuntimeProviderChatMessage, ChatRequest as RuntimeProviderChatRequest,
    ChatResponse as RuntimeProviderChatResponse, ConversationMessage as RuntimeConversationMessage,
    ProviderCapabilities as RuntimeProviderCapabilities, StreamChunk as RuntimeStreamChunk,
    StreamOptions as RuntimeStreamOptions, ToolCall as RuntimeProviderToolCall,
    ToolResultMessage as RuntimeToolResultMessage,
};
use corvus::providers::{self, Provider as RuntimeProvider};
use corvus::security::{self, Sandbox as RuntimeSandbox};
use corvus::tools::traits::{
    ToolDescriptorHint as RuntimeToolDescriptorHint,
    ToolDescriptorMcpHint as RuntimeToolDescriptorMcpHint,
    ToolDescriptorMcpPromptArgumentHint as RuntimeToolDescriptorMcpPromptArgumentHint,
    ToolSourceMetadata as RuntimeToolSourceMetadata,
};
use corvus::tools::{
    self, Tool as RuntimeTool, ToolResult as RuntimeToolResult, ToolSpec as RuntimeToolSpec,
};

struct DummySandbox;

impl corvus_traits::security::Sandbox for DummySandbox {
    fn wrap_command(&self, _cmd: &mut std::process::Command) -> std::io::Result<()> {
        Ok(())
    }

    fn is_available(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "dummy"
    }

    fn description(&self) -> &str {
        "dummy sandbox"
    }
}

struct DummyChannel;

#[async_trait]
impl corvus_traits::channels::Channel for DummyChannel {
    fn name(&self) -> &str {
        "dummy"
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<corvus_traits::channels::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

struct DummyMemory;

#[async_trait]
impl corvus_traits::memory::Memory for DummyMemory {
    fn name(&self) -> &str {
        "dummy"
    }

    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<corvus_traits::memory::MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn get(&self, _key: &str) -> anyhow::Result<Option<corvus_traits::memory::MemoryEntry>> {
        Ok(None)
    }

    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<corvus_traits::memory::MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(0)
    }

    async fn health_check(&self) -> bool {
        true
    }
}

struct DummyTool;

#[async_trait]
impl corvus_traits::tools::Tool for DummyTool {
    fn name(&self) -> &str {
        "dummy"
    }

    fn description(&self) -> &str {
        "dummy tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<RuntimeToolResult> {
        Ok(RuntimeToolResult {
            success: true,
            output: args
                .get("value")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            error: None,
            structured: None,
        })
    }
}

struct DummyProvider;

#[async_trait]
impl corvus_traits::providers::Provider for DummyProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

#[test]
fn legacy_paths_match_extracted_trait_identities() {
    assert_eq!(
        TypeId::of::<&dyn RuntimeSandbox>(),
        TypeId::of::<&dyn corvus_traits::security::Sandbox>()
    );
    assert_eq!(
        TypeId::of::<&dyn security::traits::Sandbox>(),
        TypeId::of::<&dyn corvus_traits::security::Sandbox>()
    );

    assert_eq!(
        TypeId::of::<&dyn RuntimeChannel>(),
        TypeId::of::<&dyn corvus_traits::channels::Channel>()
    );
    assert_eq!(
        TypeId::of::<&dyn channels::traits::Channel>(),
        TypeId::of::<&dyn corvus_traits::channels::Channel>()
    );

    assert_eq!(
        TypeId::of::<&dyn RuntimeMemory>(),
        TypeId::of::<&dyn corvus_traits::memory::Memory>()
    );
    assert_eq!(
        TypeId::of::<&dyn memory::traits::Memory>(),
        TypeId::of::<&dyn corvus_traits::memory::Memory>()
    );

    assert_eq!(
        TypeId::of::<&dyn RuntimeTool>(),
        TypeId::of::<&dyn corvus_traits::tools::Tool>()
    );
    assert_eq!(
        TypeId::of::<&dyn tools::traits::Tool>(),
        TypeId::of::<&dyn corvus_traits::tools::Tool>()
    );
    assert_eq!(
        TypeId::of::<RuntimeToolResult>(),
        TypeId::of::<corvus_traits::tools::ToolResult>()
    );
    assert_eq!(
        TypeId::of::<RuntimeToolSpec>(),
        TypeId::of::<corvus_traits::tools::ToolSpec>()
    );
    assert_eq!(
        TypeId::of::<RuntimeToolSourceMetadata>(),
        TypeId::of::<corvus_traits::tools::ToolSourceMetadata>()
    );
    assert_eq!(
        TypeId::of::<RuntimeToolDescriptorHint>(),
        TypeId::of::<corvus_traits::tools::ToolDescriptorHint>()
    );
    assert_eq!(
        TypeId::of::<RuntimeToolDescriptorMcpHint>(),
        TypeId::of::<corvus_traits::tools::ToolDescriptorMcpHint>()
    );
    assert_eq!(
        TypeId::of::<RuntimeToolDescriptorMcpPromptArgumentHint>(),
        TypeId::of::<corvus_traits::tools::ToolDescriptorMcpPromptArgumentHint>()
    );

    assert_eq!(
        TypeId::of::<RuntimeAllowedImageMime>(),
        TypeId::of::<corvus_traits::multimedia::AllowedImageMime>()
    );
    assert_eq!(
        TypeId::of::<media::AllowedImageMime>(),
        TypeId::of::<corvus_traits::multimedia::AllowedImageMime>()
    );
    assert_eq!(
        TypeId::of::<RuntimeImageTransportForm>(),
        TypeId::of::<corvus_traits::multimedia::ImageTransportForm>()
    );
    assert_eq!(
        TypeId::of::<RuntimeStagedImage>(),
        TypeId::of::<corvus_traits::multimedia::StagedImage>()
    );
    assert_eq!(
        TypeId::of::<RuntimeImageHistoryMeta>(),
        TypeId::of::<corvus_traits::multimedia::ImageHistoryMeta>()
    );
    assert_eq!(
        TypeId::of::<RuntimeAudioHistoryMeta>(),
        TypeId::of::<corvus_traits::multimedia::AudioHistoryMeta>()
    );
    assert_eq!(
        TypeId::of::<audio_media::AudioHistoryMeta>(),
        TypeId::of::<corvus_traits::multimedia::AudioHistoryMeta>()
    );

    assert_eq!(
        TypeId::of::<&dyn RuntimeProvider>(),
        TypeId::of::<&dyn corvus_traits::providers::Provider>()
    );
    assert_eq!(
        TypeId::of::<&dyn providers::traits::Provider>(),
        TypeId::of::<&dyn corvus_traits::providers::Provider>()
    );
    assert_eq!(
        TypeId::of::<providers::ChatMessage>(),
        TypeId::of::<corvus_traits::providers::ChatMessage>()
    );
    assert_eq!(
        TypeId::of::<RuntimeProviderChatMessage>(),
        TypeId::of::<corvus_traits::providers::ChatMessage>()
    );
    assert_eq!(
        TypeId::of::<providers::ToolCall>(),
        TypeId::of::<corvus_traits::providers::ToolCall>()
    );
    assert_eq!(
        TypeId::of::<RuntimeProviderToolCall>(),
        TypeId::of::<corvus_traits::providers::ToolCall>()
    );
    assert_eq!(
        TypeId::of::<providers::ChatResponse>(),
        TypeId::of::<corvus_traits::providers::ChatResponse>()
    );
    assert_eq!(
        TypeId::of::<RuntimeProviderChatResponse>(),
        TypeId::of::<corvus_traits::providers::ChatResponse>()
    );
    assert_eq!(
        TypeId::of::<providers::ChatRequest<'static>>(),
        TypeId::of::<corvus_traits::providers::ChatRequest<'static>>()
    );
    assert_eq!(
        TypeId::of::<RuntimeProviderChatRequest<'static>>(),
        TypeId::of::<corvus_traits::providers::ChatRequest<'static>>()
    );
    assert_eq!(
        TypeId::of::<providers::ToolResultMessage>(),
        TypeId::of::<corvus_traits::providers::ToolResultMessage>()
    );
    assert_eq!(
        TypeId::of::<RuntimeToolResultMessage>(),
        TypeId::of::<corvus_traits::providers::ToolResultMessage>()
    );
    assert_eq!(
        TypeId::of::<providers::ConversationMessage>(),
        TypeId::of::<corvus_traits::providers::ConversationMessage>()
    );
    assert_eq!(
        TypeId::of::<RuntimeConversationMessage>(),
        TypeId::of::<corvus_traits::providers::ConversationMessage>()
    );
    assert_eq!(
        TypeId::of::<providers::StreamChunk>(),
        TypeId::of::<corvus_traits::providers::StreamChunk>()
    );
    assert_eq!(
        TypeId::of::<RuntimeStreamChunk>(),
        TypeId::of::<corvus_traits::providers::StreamChunk>()
    );
    assert_eq!(
        TypeId::of::<providers::StreamOptions>(),
        TypeId::of::<corvus_traits::providers::StreamOptions>()
    );
    assert_eq!(
        TypeId::of::<RuntimeStreamOptions>(),
        TypeId::of::<corvus_traits::providers::StreamOptions>()
    );
    assert_eq!(
        TypeId::of::<providers::ProviderCapabilities>(),
        TypeId::of::<corvus_traits::providers::ProviderCapabilities>()
    );
    assert_eq!(
        TypeId::of::<RuntimeProviderCapabilities>(),
        TypeId::of::<corvus_traits::providers::ProviderCapabilities>()
    );
}

#[test]
fn legacy_paths_accept_trait_objects_from_extracted_crate() {
    let sandbox = DummySandbox;
    let sandbox_ref: &dyn RuntimeSandbox = &sandbox;
    let sandbox_traits_ref: &dyn security::traits::Sandbox = sandbox_ref;
    let sandbox_new_ref: &dyn corvus_traits::security::Sandbox = sandbox_traits_ref;
    assert_eq!(sandbox_new_ref.name(), "dummy");

    let channel = DummyChannel;
    let channel_ref: &dyn RuntimeChannel = &channel;
    let channel_traits_ref: &dyn channels::traits::Channel = channel_ref;
    let channel_new_ref: &dyn corvus_traits::channels::Channel = channel_traits_ref;
    assert_eq!(channel_new_ref.name(), "dummy");

    let memory = DummyMemory;
    let memory_ref: &dyn RuntimeMemory = &memory;
    let memory_traits_ref: &dyn memory::traits::Memory = memory_ref;
    let memory_new_ref: &dyn corvus_traits::memory::Memory = memory_traits_ref;
    assert_eq!(memory_new_ref.name(), "dummy");

    let tool = DummyTool;
    let tool_ref: &dyn RuntimeTool = &tool;
    let tool_traits_ref: &dyn tools::traits::Tool = tool_ref;
    let tool_new_ref: &dyn corvus_traits::tools::Tool = tool_traits_ref;
    assert_eq!(tool_new_ref.name(), "dummy");
    assert_eq!(tool_new_ref.spec().name, "dummy");

    let provider = DummyProvider;
    let provider_ref: &dyn RuntimeProvider = &provider;
    let provider_traits_ref: &dyn providers::traits::Provider = provider_ref;
    let provider_new_ref: &dyn corvus_traits::providers::Provider = provider_traits_ref;
    assert_eq!(
        provider_new_ref.capabilities(),
        RuntimeProviderCapabilities::default()
    );
}
