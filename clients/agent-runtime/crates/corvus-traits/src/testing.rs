/// Generates a set of contract-compliance `#[tokio::test]` functions for a [`Provider`]
/// implementation.
///
/// Each generated test calls a factory expression (`$factory`) to produce an instance,
/// then exercises a required contract method to verify the implementation compiles and
/// returns the expected type.
///
/// # Usage
///
/// ```rust,ignore
/// provider_compliance_tests!(MyProvider::new());
/// ```
#[macro_export]
macro_rules! provider_compliance_tests {
    ($factory:expr) => {
        #[tokio::test]
        async fn complies_with_capabilities_contract() {
            let instance = $factory;
            let caps = instance.capabilities();
            let _native: bool = caps.native_tool_calling;
            let _image: bool = caps.image_input;
        }

        #[tokio::test]
        async fn complies_with_chat_contract() {
            let instance = $factory;
            let result = instance
                .chat_with_system(None, "ping", "test-model", 0.0)
                .await;
            assert!(
                result.is_ok(),
                "chat_with_system must return Ok: {:?}",
                result
            );
        }

        #[tokio::test]
        async fn complies_with_streaming_contract() {
            let instance = $factory;
            let _supports: bool = instance.supports_streaming();
        }

        #[tokio::test]
        async fn complies_with_tool_conversion_contract() {
            let instance = $factory;
            let payload = instance.convert_tools(&[]);
            match payload {
                $crate::providers::ToolsPayload::Gemini { .. }
                | $crate::providers::ToolsPayload::Anthropic { .. }
                | $crate::providers::ToolsPayload::OpenAI { .. }
                | $crate::providers::ToolsPayload::PromptGuided { .. } => {}
            }
        }
    };
}

/// Generates contract-compliance `#[tokio::test]` functions for a [`Tool`] implementation.
///
/// # Usage
///
/// ```rust,ignore
/// tool_compliance_tests!(MyTool::new());
/// ```
#[macro_export]
macro_rules! tool_compliance_tests {
    ($factory:expr) => {
        #[tokio::test]
        async fn complies_with_name_contract() {
            let instance = $factory;
            assert!(!instance.name().is_empty(), "name() must be non-empty");
        }

        #[tokio::test]
        async fn complies_with_description_contract() {
            let instance = $factory;
            assert!(
                !instance.description().is_empty(),
                "description() must be non-empty"
            );
        }

        #[tokio::test]
        async fn complies_with_execute_contract() {
            let instance = $factory;
            let result = instance.execute(serde_json::json!({})).await;
            assert!(result.is_ok(), "execute must return Ok: {:?}", result);
        }
    };
}

/// Generates contract-compliance `#[tokio::test]` functions for a [`Memory`] implementation.
///
/// # Usage
///
/// ```rust,ignore
/// memory_compliance_tests!(MyMemory::new());
/// ```
#[macro_export]
macro_rules! memory_compliance_tests {
    ($factory:expr) => {
        #[tokio::test]
        async fn complies_with_name_contract() {
            let instance = $factory;
            assert!(!instance.name().is_empty(), "name() must be non-empty");
        }

        #[tokio::test]
        async fn complies_with_health_check_contract() {
            let instance = $factory;
            let _healthy: bool = instance.health_check().await;
        }

        #[tokio::test]
        async fn complies_with_count_contract() {
            let instance = $factory;
            let result = instance.count().await;
            assert!(result.is_ok(), "count() must return Ok: {:?}", result);
        }
    };
}

/// Generates contract-compliance `#[tokio::test]` functions for a [`Channel`] implementation.
///
/// # Usage
///
/// ```rust,ignore
/// channel_compliance_tests!(MyChannel::new());
/// ```
#[macro_export]
macro_rules! channel_compliance_tests {
    ($factory:expr) => {
        #[tokio::test]
        async fn complies_with_name_contract() {
            let instance = $factory;
            assert!(!instance.name().is_empty(), "name() must be non-empty");
        }

        #[tokio::test]
        async fn complies_with_health_check_contract() {
            let instance = $factory;
            let _healthy: bool = instance.health_check().await;
        }
    };
}

/// Generates contract-compliance `#[test]` functions for a [`Sandbox`] implementation.
///
/// # Usage
///
/// ```rust,ignore
/// sandbox_compliance_tests!(MySandbox::new());
/// ```
#[macro_export]
macro_rules! sandbox_compliance_tests {
    ($factory:expr) => {
        #[test]
        fn complies_with_name_contract() {
            let instance = $factory;
            assert!(!instance.name().is_empty(), "name() must be non-empty");
        }

        #[test]
        fn complies_with_availability_contract() {
            let instance = $factory;
            let _available: bool = instance.is_available();
        }
    };
}

#[cfg(test)]
mod tests {
    use super::super::channels::{Channel, ChannelMessage, SendMessage};
    use super::super::memory::{Memory, MemoryCategory, MemoryEntry};
    use super::super::providers::{Provider, ProviderCapabilities};
    use super::super::security::Sandbox;
    use super::super::tools::{Tool, ToolResult};
    use async_trait::async_trait;

    // -------------------------------------------------------------------------
    // Stub implementations
    // -------------------------------------------------------------------------

    struct StubProvider;

    #[async_trait]
    impl Provider for StubProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok("stub".into())
        }
    }

    struct StubTool;

    #[async_trait]
    impl Tool for StubTool {
        fn name(&self) -> &str {
            "stub"
        }

        fn description(&self) -> &str {
            "stub tool"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {}})
        }

        async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
            Ok(ToolResult {
                success: true,
                output: "stub".into(),
                error: None,
                structured: None,
            })
        }
    }

    struct StubMemory;

    #[async_trait]
    impl Memory for StubMemory {
        fn name(&self) -> &str {
            "stub"
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
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(vec![])
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(vec![])
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

    struct StubChannel;

    #[async_trait]
    impl Channel for StubChannel {
        fn name(&self) -> &str {
            "stub"
        }

        async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
            Ok(())
        }

        async fn listen(
            &self,
            _tx: tokio::sync::mpsc::Sender<ChannelMessage>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct StubSandbox;

    impl Sandbox for StubSandbox {
        fn wrap_command(&self, _cmd: &mut std::process::Command) -> std::io::Result<()> {
            Ok(())
        }

        fn is_available(&self) -> bool {
            true
        }

        fn name(&self) -> &str {
            "stub"
        }

        fn description(&self) -> &str {
            "stub sandbox"
        }
    }

    // -------------------------------------------------------------------------
    // Macro invocations — each macro generates its compliance test functions.
    // Name collisions are avoided by invoking each macro in its own submodule.
    // -------------------------------------------------------------------------

    mod provider_compliance {
        use super::*;
        provider_compliance_tests!(StubProvider);
    }

    mod tool_compliance {
        use super::*;
        tool_compliance_tests!(StubTool);
    }

    mod memory_compliance {
        use super::*;
        memory_compliance_tests!(StubMemory);
    }

    mod channel_compliance {
        use super::*;
        channel_compliance_tests!(StubChannel);
    }

    mod sandbox_compliance {
        use super::*;
        sandbox_compliance_tests!(StubSandbox);
    }

    // -------------------------------------------------------------------------
    // Sanity check: ProviderCapabilities fields are accessible
    // -------------------------------------------------------------------------

    #[test]
    fn provider_capabilities_has_expected_fields() {
        let caps = ProviderCapabilities::default();
        let _native: bool = caps.native_tool_calling;
        let _image: bool = caps.image_input;
    }
}
