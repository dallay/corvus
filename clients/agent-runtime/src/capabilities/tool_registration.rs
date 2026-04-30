use super::descriptor::{
    ActivationMode, CapabilityCompatibility, CapabilityDependencies, CapabilityDescriptor,
    CapabilityError, CapabilityFamily, CapabilityKind, CapabilityLifecycle, CapabilityMetadata,
    CapabilitySecurity, DiscoveryMode, McpCapabilityMetadata, PromptArgumentDescriptor,
    SourceClassification, TeardownMode, ENTRYPOINT_AGENT, ENTRYPOINT_CHANNELS, ENTRYPOINT_GATEWAY,
    M2_CAPABILITY_VERSION, TOOL_RUNTIME_CONTRACT,
};
use super::registry::CapabilityRegistry;
use crate::tools::mcp::{
    adapter::McpToolAdapter, prompt_adapter::McpPromptAdapter, resource_adapter::McpResourceAdapter,
};
use crate::tools::traits::{Tool, ToolDescriptorHint, ToolSpec};

pub fn build_registry_from_tools(
    tools: &[Box<dyn Tool>],
) -> Result<CapabilityRegistry, CapabilityError> {
    let descriptors = tools
        .iter()
        .map(|tool| build_tool_descriptor(tool.as_ref()))
        .collect::<Result<Vec<_>, _>>()?;
    CapabilityRegistry::from_descriptors(descriptors)
}

pub fn build_tool_descriptor(tool: &dyn Tool) -> Result<CapabilityDescriptor, CapabilityError> {
    build_descriptor_from_parts(tool.spec(), tool.descriptor_hint())
}

pub fn build_native_tool_descriptor(
    tool: &dyn Tool,
) -> Result<CapabilityDescriptor, CapabilityError> {
    build_tool_descriptor(tool)
}

pub fn build_mcp_tool_descriptor(
    adapter: &McpToolAdapter,
) -> Result<CapabilityDescriptor, CapabilityError> {
    build_tool_descriptor(adapter)
}

pub fn build_mcp_resource_descriptor(
    adapter: &McpResourceAdapter,
) -> Result<CapabilityDescriptor, CapabilityError> {
    build_tool_descriptor(adapter)
}

pub fn build_mcp_prompt_descriptor(
    adapter: &McpPromptAdapter,
) -> Result<CapabilityDescriptor, CapabilityError> {
    build_tool_descriptor(adapter)
}

fn build_descriptor_from_parts(
    spec: ToolSpec,
    hint: ToolDescriptorHint,
) -> Result<CapabilityDescriptor, CapabilityError> {
    let id = spec.name.clone();
    let (namespace, source_classification) = classify_descriptor(&spec)?;
    let mcp_metadata = build_mcp_metadata(&spec, &hint, namespace);

    Ok(CapabilityDescriptor {
        id: id.clone(),
        namespace: namespace.to_string(),
        version: M2_CAPABILITY_VERSION.to_string(),
        family: CapabilityFamily::Tool,
        kind: CapabilityKind::Executable,
        dependencies: CapabilityDependencies::default(),
        lifecycle: CapabilityLifecycle {
            discovery_mode: if namespace == "native.tool" {
                DiscoveryMode::Static
            } else {
                DiscoveryMode::Discovered
            },
            activation_mode: ActivationMode::RuntimeWired,
            teardown_mode: Some(TeardownMode::None),
        },
        security: CapabilitySecurity {
            policy_scope: "tool".to_string(),
            audit_namespace: id,
            source_classification,
            risk_tags: vec![],
        },
        compatibility: CapabilityCompatibility {
            runtime_contracts: vec![TOOL_RUNTIME_CONTRACT.to_string()],
            entrypoint_parity_scope: vec![
                ENTRYPOINT_AGENT.to_string(),
                ENTRYPOINT_CHANNELS.to_string(),
                ENTRYPOINT_GATEWAY.to_string(),
            ],
        },
        metadata: CapabilityMetadata {
            description: spec.description,
            parameters_schema: spec.parameters,
            source: spec.source,
            mcp: mcp_metadata,
            aliases: spec.aliases,
        },
    })
}

fn classify_descriptor(
    spec: &ToolSpec,
) -> Result<(&'static str, SourceClassification), CapabilityError> {
    match spec.source.as_ref().map(|source| source.kind.as_str()) {
        None => Ok(("native.tool", SourceClassification::Native)),
        Some("mcp") => Ok(("mcp.tool", SourceClassification::Mcp)),
        Some("mcp_resource") => Ok(("mcp.resource", SourceClassification::McpResource)),
        Some("mcp_prompt") => Ok(("mcp.prompt", SourceClassification::McpPrompt)),
        Some(kind) => Err(CapabilityError::InvalidMetadata {
            id: spec.name.clone(),
            reason: format!("unsupported tool source kind '{kind}'"),
        }),
    }
}

fn build_mcp_metadata(
    spec: &ToolSpec,
    hint: &ToolDescriptorHint,
    namespace: &str,
) -> Option<McpCapabilityMetadata> {
    let source = spec.source.as_ref()?;
    let mcp = hint.mcp.as_ref();
    let server = mcp
        .and_then(|value| value.server.clone())
        .or_else(|| source.server.clone())?;
    let upstream_name = mcp
        .and_then(|value| value.upstream_name.clone())
        .or_else(|| source.original_name.clone());
    let resource_uri = mcp
        .and_then(|value| value.resource_uri.clone())
        .or_else(|| {
            (namespace == "mcp.resource")
                .then(|| source.original_name.clone())
                .flatten()
        });
    let mime_type = mcp.and_then(|value| value.mime_type.clone());
    let prompt_arguments = mcp
        .map(|value| {
            value
                .prompt_arguments
                .iter()
                .map(|argument| PromptArgumentDescriptor {
                    name: argument.name.clone(),
                    description: argument.description.clone(),
                    required: argument.required,
                })
                .collect()
        })
        .unwrap_or_default();

    Some(McpCapabilityMetadata {
        server,
        upstream_name,
        resource_uri,
        mime_type,
        prompt_arguments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::McpServerConfig;
    use crate::tools::mcp::client::{
        McpClient, McpPromptManifest, McpResourceManifest, McpToolManifest, PromptArgument,
    };
    use crate::tools::traits::{
        ToolDescriptorMcpHint, ToolDescriptorMcpPromptArgumentHint, ToolResult,
    };
    use async_trait::async_trait;

    struct NativeTestTool;

    #[async_trait]
    impl Tool for NativeTestTool {
        fn name(&self) -> &str {
            "shell"
        }
        fn description(&self) -> &str {
            "Execute a shell command"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {"command": {"type": "string"}}})
        }
        fn spec(&self) -> ToolSpec {
            ToolSpec {
                name: self.name().to_string(),
                description: self.description().to_string(),
                parameters: self.parameters_schema(),
                source: None,
                aliases: vec!["sh".to_string()],
            }
        }
        async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
            Ok(ToolResult {
                success: true,
                output: String::new(),
                error: None,
                structured: None,
            })
        }
    }

    fn test_server(name: &str) -> McpServerConfig {
        McpServerConfig {
            name: name.into(),
            command: "__mcp_mock__".into(),
            ..McpServerConfig::default()
        }
    }

    fn test_client(server: &McpServerConfig) -> McpClient {
        McpClient::new(server.clone())
    }

    #[test]
    fn builds_native_tool_descriptor_with_defaults() {
        let descriptor = build_native_tool_descriptor(&NativeTestTool).unwrap();

        assert_eq!(descriptor.id, "shell");
        assert_eq!(descriptor.namespace, "native.tool");
        assert_eq!(descriptor.version, M2_CAPABILITY_VERSION);
        assert_eq!(descriptor.family, CapabilityFamily::Tool);
        assert_eq!(descriptor.kind, CapabilityKind::Executable);
        assert!(descriptor.dependencies.required.is_empty());
        assert!(descriptor.dependencies.optional.is_empty());
        assert_eq!(descriptor.lifecycle.discovery_mode, DiscoveryMode::Static);
        assert_eq!(
            descriptor.security.source_classification,
            SourceClassification::Native
        );
        assert!(descriptor.metadata.mcp.is_none());
    }

    #[test]
    fn native_tool_descriptor_preserves_aliases() {
        let descriptor = build_native_tool_descriptor(&NativeTestTool).unwrap();

        assert_eq!(descriptor.metadata.aliases, vec!["sh"]);
    }

    #[test]
    fn registry_from_tools_preserves_canonical_and_alias_metadata_without_duplicate_entries() {
        let registry = build_registry_from_tools(&[Box::new(NativeTestTool)]).unwrap();

        assert_eq!(registry.len(), 1);
        let descriptor = registry.get("shell").expect("descriptor present");
        assert_eq!(descriptor.id, "shell");
        assert_eq!(descriptor.metadata.aliases, vec!["sh"]);
    }

    #[test]
    fn builds_mcp_tool_descriptor_with_canonical_metadata() {
        let server = test_server("docs");
        let adapter = McpToolAdapter::from_manifest(
            &server,
            McpToolManifest {
                name: "search".into(),
                description: "Search docs".into(),
                parameters: serde_json::json!({"type":"object","properties":{}}),
            },
            test_client(&server),
        )
        .unwrap();

        let descriptor = build_mcp_tool_descriptor(&adapter).unwrap();

        assert_eq!(descriptor.id, "mcp.docs.search");
        assert_eq!(descriptor.namespace, "mcp.tool");
        assert_eq!(
            descriptor.security.source_classification,
            SourceClassification::Mcp
        );
        assert_eq!(descriptor.metadata.mcp.as_ref().unwrap().server, "docs");
        assert_eq!(
            descriptor
                .metadata
                .mcp
                .as_ref()
                .unwrap()
                .upstream_name
                .as_deref(),
            Some("search")
        );
    }

    #[test]
    fn builds_mcp_resource_descriptor_with_uri_and_mime_type() {
        let server = test_server("docs");
        let adapter = McpResourceAdapter::from_manifest(
            &server,
            McpResourceManifest {
                name: "api-spec".into(),
                uri: "docs://api-spec".into(),
                description: "API specification".into(),
                mime_type: Some("text/markdown".into()),
            },
            test_client(&server),
        )
        .unwrap();

        let descriptor = build_mcp_resource_descriptor(&adapter).unwrap();

        assert_eq!(descriptor.id, "mcp.docs.resource.api-spec");
        assert_eq!(descriptor.namespace, "mcp.resource");
        assert_eq!(
            descriptor.security.source_classification,
            SourceClassification::McpResource
        );
        let mcp = descriptor.metadata.mcp.unwrap();
        assert_eq!(mcp.server, "docs");
        assert_eq!(mcp.upstream_name.as_deref(), Some("api-spec"));
        assert_eq!(mcp.resource_uri.as_deref(), Some("docs://api-spec"));
        assert_eq!(mcp.mime_type.as_deref(), Some("text/markdown"));
    }

    #[test]
    fn builds_mcp_prompt_descriptor_with_arguments() {
        let server = test_server("workflows");
        let adapter = McpPromptAdapter::from_manifest(
            &server,
            McpPromptManifest {
                name: "code-review".into(),
                description: "Review template".into(),
                arguments: vec![PromptArgument {
                    name: "language".into(),
                    description: "Programming language".into(),
                    required: true,
                }],
            },
            test_client(&server),
        )
        .unwrap();

        let descriptor = build_mcp_prompt_descriptor(&adapter).unwrap();

        assert_eq!(descriptor.id, "mcp.workflows.prompt.code-review");
        assert_eq!(descriptor.namespace, "mcp.prompt");
        assert_eq!(
            descriptor.security.source_classification,
            SourceClassification::McpPrompt
        );
        let mcp = descriptor.metadata.mcp.unwrap();
        assert_eq!(mcp.server, "workflows");
        assert_eq!(mcp.upstream_name.as_deref(), Some("code-review"));
        assert_eq!(mcp.prompt_arguments.len(), 1);
        assert_eq!(mcp.prompt_arguments[0].name, "language");
        assert!(mcp.prompt_arguments[0].required);
    }

    #[test]
    fn build_tool_descriptor_uses_hint_when_source_metadata_is_not_enough() {
        let spec = ToolSpec {
            name: "mcp.docs.prompt.code-review".into(),
            description: "Review template".into(),
            parameters: serde_json::json!({"type":"object","properties":{}}),
            source: Some(crate::tools::traits::ToolSourceMetadata {
                kind: "mcp_prompt".into(),
                provider: Some("mcp".into()),
                server: Some("docs".into()),
                original_name: Some("code-review".into()),
            }),
            aliases: vec![],
        };
        let hint = ToolDescriptorHint {
            mcp: Some(ToolDescriptorMcpHint {
                server: Some("docs".into()),
                upstream_name: Some("code-review".into()),
                resource_uri: None,
                mime_type: None,
                prompt_arguments: vec![ToolDescriptorMcpPromptArgumentHint {
                    name: "language".into(),
                    description: "Programming language".into(),
                    required: true,
                }],
            }),
        };

        let descriptor = build_descriptor_from_parts(spec, hint).unwrap();
        assert_eq!(descriptor.metadata.mcp.unwrap().prompt_arguments.len(), 1);
    }
}
