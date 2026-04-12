use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectedCapability {
    pub key: String,
    pub config: Option<toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeSettings {
    pub profile: Option<String>,
    pub max_tool_iterations: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemorySettings {
    pub auto_save: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdentitySettings {
    pub format: Option<String>,
    pub aieos_path: Option<String>,
    pub aieos_inline: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentMetadata {
    pub name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComposedRuntimePlan {
    pub agent: AgentMetadata,
    pub provider: SelectedCapability,
    pub channels: Vec<SelectedCapability>,
    pub default_channel: Option<String>,
    pub tools: Vec<SelectedCapability>,
    pub memory: SelectedCapability,
    pub memory_settings: MemorySettings,
    pub observers: Vec<SelectedCapability>,
    pub security: SelectedCapability,
    pub tool_restrictions: Vec<String>,
    pub runtime: RuntimeSettings,
    pub identity: IdentitySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityReport {
    pub providers: Vec<String>,
    pub channels: Vec<String>,
    pub tools: Vec<String>,
    pub memory_backend: Option<String>,
    pub observers: Vec<String>,
    pub sandbox: Option<String>,
}

impl From<&ComposedRuntimePlan> for CapabilityReport {
    fn from(plan: &ComposedRuntimePlan) -> Self {
        Self {
            providers: vec![plan.provider.key.clone()],
            channels: plan.channels.iter().map(|item| item.key.clone()).collect(),
            tools: plan.tools.iter().map(|item| item.key.clone()).collect(),
            memory_backend: Some(plan.memory.key.clone()),
            observers: plan.observers.iter().map(|item| item.key.clone()).collect(),
            sandbox: Some(plan.security.key.clone()),
        }
    }
}

pub type CapabilityConfigMap = BTreeMap<String, toml::Value>;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_plan(
        provider_key: &str,
        channel_keys: &[&str],
        tool_keys: &[&str],
        memory_key: &str,
        observer_keys: &[&str],
        security_key: &str,
    ) -> ComposedRuntimePlan {
        ComposedRuntimePlan {
            agent: AgentMetadata {
                name: "test-agent".to_string(),
                description: None,
                model: None,
                temperature: None,
            },
            provider: SelectedCapability {
                key: provider_key.to_string(),
                config: None,
            },
            channels: channel_keys
                .iter()
                .map(|k| SelectedCapability {
                    key: k.to_string(),
                    config: None,
                })
                .collect(),
            default_channel: channel_keys.first().map(|k| k.to_string()),
            tools: tool_keys
                .iter()
                .map(|k| SelectedCapability {
                    key: k.to_string(),
                    config: None,
                })
                .collect(),
            memory: SelectedCapability {
                key: memory_key.to_string(),
                config: None,
            },
            memory_settings: MemorySettings { auto_save: None },
            observers: observer_keys
                .iter()
                .map(|k| SelectedCapability {
                    key: k.to_string(),
                    config: None,
                })
                .collect(),
            security: SelectedCapability {
                key: security_key.to_string(),
                config: None,
            },
            tool_restrictions: vec![],
            runtime: RuntimeSettings {
                profile: None,
                max_tool_iterations: None,
            },
            identity: IdentitySettings {
                format: None,
                aieos_path: None,
                aieos_inline: None,
            },
        }
    }

    // --- CapabilityReport::from ---

    #[test]
    fn capability_report_from_plan_has_correct_provider() {
        let plan = make_plan("anthropic", &["stdio"], &[], "sqlite", &[], "none");
        let report = CapabilityReport::from(&plan);
        assert_eq!(report.providers, vec!["anthropic"]);
    }

    #[test]
    fn capability_report_from_plan_maps_all_channels() {
        let plan = make_plan(
            "anthropic",
            &["stdio", "telegram", "discord"],
            &[],
            "none",
            &[],
            "none",
        );
        let report = CapabilityReport::from(&plan);
        assert_eq!(report.channels, vec!["stdio", "telegram", "discord"]);
    }

    #[test]
    fn capability_report_from_plan_maps_all_tools() {
        let plan = make_plan(
            "anthropic",
            &["stdio"],
            &["shell", "file_read", "browser"],
            "none",
            &[],
            "none",
        );
        let report = CapabilityReport::from(&plan);
        assert_eq!(report.tools, vec!["shell", "file_read", "browser"]);
    }

    #[test]
    fn capability_report_from_plan_always_has_memory_backend() {
        let plan = make_plan("anthropic", &["stdio"], &[], "sqlite", &[], "none");
        let report = CapabilityReport::from(&plan);
        assert_eq!(report.memory_backend, Some("sqlite".to_string()));
    }

    #[test]
    fn capability_report_from_plan_maps_observers() {
        let plan = make_plan("anthropic", &["stdio"], &[], "none", &["log", "prometheus"], "none");
        let report = CapabilityReport::from(&plan);
        assert_eq!(report.observers, vec!["log", "prometheus"]);
    }

    #[test]
    fn capability_report_from_plan_empty_observers_produces_empty_vec() {
        let plan = make_plan("anthropic", &["stdio"], &[], "none", &[], "none");
        let report = CapabilityReport::from(&plan);
        assert!(report.observers.is_empty());
    }

    #[test]
    fn capability_report_from_plan_always_has_sandbox() {
        let plan = make_plan("anthropic", &["stdio"], &[], "none", &[], "landlock");
        let report = CapabilityReport::from(&plan);
        assert_eq!(report.sandbox, Some("landlock".to_string()));
    }

    #[test]
    fn capability_report_from_plan_empty_tools_produces_empty_vec() {
        let plan = make_plan("anthropic", &["stdio"], &[], "none", &[], "none");
        let report = CapabilityReport::from(&plan);
        assert!(report.tools.is_empty());
    }

    // --- SelectedCapability ---

    #[test]
    fn selected_capability_equality() {
        let a = SelectedCapability {
            key: "shell".to_string(),
            config: None,
        };
        let b = SelectedCapability {
            key: "shell".to_string(),
            config: None,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn selected_capability_inequality_on_key() {
        let a = SelectedCapability {
            key: "shell".to_string(),
            config: None,
        };
        let b = SelectedCapability {
            key: "file_read".to_string(),
            config: None,
        };
        assert_ne!(a, b);
    }

    // --- RuntimeSettings ---

    #[test]
    fn runtime_settings_with_all_fields() {
        let settings = RuntimeSettings {
            profile: Some("code".to_string()),
            max_tool_iterations: Some(10),
        };
        assert_eq!(settings.profile.as_deref(), Some("code"));
        assert_eq!(settings.max_tool_iterations, Some(10));
    }

    // Boundary: provider is always a single entry in the report
    #[test]
    fn capability_report_providers_is_single_element_vec() {
        let plan = make_plan("openai", &["stdio"], &[], "none", &[], "none");
        let report = CapabilityReport::from(&plan);
        assert_eq!(report.providers.len(), 1);
        assert_eq!(report.providers[0], "openai");
    }
}