use super::descriptor::{CapabilityDescriptor, CapabilityError, CapabilityFamily, CapabilityKind};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct CapabilityRegistry {
    descriptors: Vec<CapabilityDescriptor>,
    by_id: BTreeMap<String, usize>,
}

impl CapabilityRegistry {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_descriptors(
        descriptors: Vec<CapabilityDescriptor>,
    ) -> Result<Self, CapabilityError> {
        let mut registry = Self::empty();
        for descriptor in descriptors {
            registry.register(descriptor)?;
        }
        Ok(registry)
    }

    pub fn register(&mut self, descriptor: CapabilityDescriptor) -> Result<(), CapabilityError> {
        Self::validate_descriptor(&descriptor)?;
        if self.by_id.contains_key(&descriptor.id) {
            return Err(CapabilityError::DuplicateId {
                id: descriptor.id.clone(),
            });
        }

        let index = self.descriptors.len();
        self.by_id.insert(descriptor.id.clone(), index);
        self.descriptors.push(descriptor);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&CapabilityDescriptor> {
        self.by_id
            .get(id)
            .and_then(|index| self.descriptors.get(*index))
    }

    pub fn iter(&self) -> impl Iterator<Item = &CapabilityDescriptor> {
        self.descriptors.iter()
    }

    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }

    pub fn validate_descriptor(descriptor: &CapabilityDescriptor) -> Result<(), CapabilityError> {
        let id = Some(descriptor.id.as_str());
        validate_required_fields(descriptor, id)?;
        validate_metadata_shape(descriptor)?;
        validate_m2_contract(descriptor)?;
        validate_namespace(descriptor)?;
        validate_mcp_metadata(descriptor, id)?;

        Ok(())
    }
}

fn validate_required_fields(
    descriptor: &CapabilityDescriptor,
    id: Option<&str>,
) -> Result<(), CapabilityError> {
    for (value, field, field_id) in [
        (descriptor.id.trim(), "id", None),
        (descriptor.namespace.trim(), "namespace", id),
        (descriptor.version.trim(), "version", id),
        (
            descriptor.metadata.description.trim(),
            "metadata.description",
            id,
        ),
        (
            descriptor.security.policy_scope.trim(),
            "security.policy_scope",
            id,
        ),
        (
            descriptor.security.audit_namespace.trim(),
            "security.audit_namespace",
            id,
        ),
    ] {
        if value.is_empty() {
            return Err(CapabilityError::missing_field(field, field_id));
        }
    }

    for (items, field) in [
        (
            descriptor.compatibility.runtime_contracts.as_slice(),
            "compatibility.runtime_contracts",
        ),
        (
            descriptor.compatibility.entrypoint_parity_scope.as_slice(),
            "compatibility.entrypoint_parity_scope",
        ),
    ] {
        if items.is_empty() {
            return Err(CapabilityError::missing_field(field, id));
        }
        if items.iter().any(|item| item.trim().is_empty()) {
            return Err(CapabilityError::missing_field(field, id));
        }
    }

    Ok(())
}

fn validate_metadata_shape(descriptor: &CapabilityDescriptor) -> Result<(), CapabilityError> {
    if descriptor.metadata.parameters_schema.is_object() {
        return Ok(());
    }

    Err(CapabilityError::InvalidMetadata {
        id: descriptor.id.clone(),
        reason: "parameters schema must be a JSON object".to_string(),
    })
}

fn validate_m2_contract(descriptor: &CapabilityDescriptor) -> Result<(), CapabilityError> {
    if descriptor.family != CapabilityFamily::Tool {
        return Err(CapabilityError::InvalidFamilyForM2 {
            id: descriptor.id.clone(),
            family: format!("{:?}", descriptor.family),
        });
    }
    if descriptor.kind != CapabilityKind::Executable {
        return Err(CapabilityError::InvalidKindForM2 {
            id: descriptor.id.clone(),
            kind: format!("{:?}", descriptor.kind),
        });
    }
    Ok(())
}

fn validate_namespace(descriptor: &CapabilityDescriptor) -> Result<(), CapabilityError> {
    let valid_namespace = matches!(
        descriptor.namespace.as_str(),
        "native.tool" | "mcp.tool" | "mcp.resource" | "mcp.prompt"
    );
    if valid_namespace {
        Ok(())
    } else {
        Err(CapabilityError::InvalidNamespace {
            id: descriptor.id.clone(),
            namespace: descriptor.namespace.clone(),
        })
    }
}

fn validate_mcp_metadata(
    descriptor: &CapabilityDescriptor,
    id: Option<&str>,
) -> Result<(), CapabilityError> {
    if !descriptor.namespace.starts_with("mcp.") {
        return Ok(());
    }

    let mcp = descriptor
        .metadata
        .mcp
        .as_ref()
        .ok_or_else(|| CapabilityError::missing_field("metadata.mcp", id))?;
    if mcp.server.trim().is_empty() {
        return Err(CapabilityError::missing_field("metadata.mcp.server", id));
    }
    if descriptor.namespace == "mcp.resource"
        && mcp.resource_uri.as_deref().unwrap_or("").trim().is_empty()
    {
        return Err(CapabilityError::missing_field(
            "metadata.mcp.resource_uri",
            id,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::descriptor::{
        ActivationMode, CapabilityCompatibility, CapabilityDependencies, CapabilityLifecycle,
        CapabilityMetadata, CapabilitySecurity, DiscoveryMode, SourceClassification, TeardownMode,
    };

    fn descriptor(id: &str, namespace: &str) -> CapabilityDescriptor {
        CapabilityDescriptor {
            id: id.to_string(),
            namespace: namespace.to_string(),
            version: "1.0.0".to_string(),
            family: CapabilityFamily::Tool,
            kind: CapabilityKind::Executable,
            dependencies: CapabilityDependencies::default(),
            lifecycle: CapabilityLifecycle {
                discovery_mode: DiscoveryMode::Static,
                activation_mode: ActivationMode::RuntimeWired,
                teardown_mode: Some(TeardownMode::None),
            },
            security: CapabilitySecurity {
                policy_scope: "tool".to_string(),
                audit_namespace: id.to_string(),
                source_classification: SourceClassification::Native,
                risk_tags: vec![],
            },
            compatibility: CapabilityCompatibility {
                runtime_contracts: vec!["tool-trait-v1".to_string()],
                entrypoint_parity_scope: vec!["agent".to_string()],
            },
            metadata: CapabilityMetadata {
                description: "desc".to_string(),
                parameters_schema: serde_json::json!({"type": "object", "properties": {}}),
                source: None,
                mcp: None,
                aliases: vec![],
            },
        }
    }

    #[test]
    fn rejects_missing_required_fields() {
        let mut descriptor = descriptor("shell", "native.tool");
        descriptor.version.clear();

        let error = CapabilityRegistry::validate_descriptor(&descriptor).unwrap_err();
        assert!(matches!(error, CapabilityError::MissingField { .. }));
        assert!(error.to_string().contains("version"));
    }

    #[test]
    fn rejects_invalid_namespace() {
        let descriptor = descriptor("shell", "tool");

        let error = CapabilityRegistry::validate_descriptor(&descriptor).unwrap_err();
        assert!(matches!(error, CapabilityError::InvalidNamespace { .. }));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let mut registry = CapabilityRegistry::empty();
        registry
            .register(descriptor("shell", "native.tool"))
            .unwrap();
        let error = registry
            .register(descriptor("shell", "native.tool"))
            .unwrap_err();

        assert_eq!(error, CapabilityError::DuplicateId { id: "shell".into() });
    }

    #[test]
    fn preserves_successful_registration_order() {
        let registry = CapabilityRegistry::from_descriptors(vec![
            descriptor("file_read", "native.tool"),
            descriptor("shell", "native.tool"),
        ])
        .unwrap();
        let ids: Vec<&str> = registry
            .iter()
            .map(|descriptor| descriptor.id.as_str())
            .collect();

        assert_eq!(ids, vec!["file_read", "shell"]);
    }

    #[test]
    fn explicit_collision_policy_is_duplicate_error_for_same_visible_id() {
        let mut mcp_descriptor = descriptor("shell", "mcp.tool");
        mcp_descriptor.metadata.source = Some(crate::tools::traits::ToolSourceMetadata {
            kind: "mcp".into(),
            provider: Some("mcp".into()),
            server: Some("docs".into()),
            original_name: Some("shell".into()),
        });
        mcp_descriptor.metadata.mcp =
            Some(crate::capabilities::descriptor::McpCapabilityMetadata {
                server: "docs".into(),
                upstream_name: Some("shell".into()),
                resource_uri: None,
                mime_type: None,
                prompt_arguments: vec![],
            });

        let registry = CapabilityRegistry::from_descriptors(vec![
            descriptor("shell", "native.tool"),
            mcp_descriptor,
        ]);

        let error = registry.unwrap_err();
        assert_eq!(error, CapabilityError::DuplicateId { id: "shell".into() });
    }

    #[test]
    fn rejects_whitespace_only_compatibility_entries() {
        let mut descriptor = descriptor("shell", "native.tool");
        descriptor.compatibility.runtime_contracts = vec!["   ".to_string()];

        let error = CapabilityRegistry::validate_descriptor(&descriptor).unwrap_err();

        assert!(matches!(error, CapabilityError::MissingField { .. }));
        assert!(error
            .to_string()
            .contains("compatibility.runtime_contracts"));

        descriptor.compatibility.runtime_contracts = vec!["tool-trait-v1".to_string()];
        descriptor.compatibility.entrypoint_parity_scope = vec!["   ".to_string()];

        let parity_error = CapabilityRegistry::validate_descriptor(&descriptor).unwrap_err();

        assert!(matches!(parity_error, CapabilityError::MissingField { .. }));
        assert!(parity_error
            .to_string()
            .contains("compatibility.entrypoint_parity_scope"));
    }

    #[test]
    fn rejects_non_object_parameter_schema() {
        let mut descriptor = descriptor("shell", "native.tool");
        descriptor.metadata.parameters_schema = serde_json::json!(["oops"]);

        let error = CapabilityRegistry::validate_descriptor(&descriptor).unwrap_err();

        assert!(matches!(error, CapabilityError::InvalidMetadata { .. }));
        assert!(error
            .to_string()
            .contains("parameters schema must be a JSON object"));
    }

    #[test]
    fn rejects_missing_mcp_metadata_requirements() {
        let mut descriptor = descriptor("resource", "mcp.resource");

        let missing_metadata = CapabilityRegistry::validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(
            missing_metadata,
            CapabilityError::missing_field("metadata.mcp", Some("resource"))
        );

        descriptor.metadata.mcp = Some(crate::capabilities::descriptor::McpCapabilityMetadata {
            server: "   ".into(),
            upstream_name: Some("resource".into()),
            resource_uri: None,
            mime_type: None,
            prompt_arguments: vec![],
        });

        let blank_server = CapabilityRegistry::validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(
            blank_server,
            CapabilityError::missing_field("metadata.mcp.server", Some("resource"))
        );

        descriptor.metadata.mcp = Some(crate::capabilities::descriptor::McpCapabilityMetadata {
            server: "docs".into(),
            upstream_name: Some("resource".into()),
            resource_uri: Some("   ".into()),
            mime_type: None,
            prompt_arguments: vec![],
        });

        let missing_resource_uri =
            CapabilityRegistry::validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(
            missing_resource_uri,
            CapabilityError::missing_field("metadata.mcp.resource_uri", Some("resource"))
        );
    }
}
