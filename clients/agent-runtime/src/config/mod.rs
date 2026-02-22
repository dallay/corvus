pub mod schema;

#[allow(unused_imports)]
pub use schema::{
    AgentConfig, AuditConfig, AutonomyConfig, BrowserComputerUseConfig, BrowserConfig,
    ChannelsConfig, ClassificationRule, ComposioConfig, Config, CostConfig, CronConfig,
    DelegateAgentConfig, DiscordConfig, DockerRuntimeConfig, GatewayConfig, HardwareConfig,
    HardwareTransport, HeartbeatConfig, HttpRequestConfig, IMessageConfig, IdentityConfig,
    LarkConfig, MatrixConfig, MemoryConfig, ModelRouteConfig, ObservabilityConfig,
    PeripheralBoardConfig, PeripheralsConfig, PluginRevocationConfig, PluginSourceConfig,
    PluginsConfig, QueryClassificationConfig, ReliabilityConfig, ResourceLimitsConfig,
    RuntimeConfig, SandboxBackend, SandboxConfig, SchedulerConfig, SecretsConfig, SecurityConfig,
    SlackConfig, StreamMode, SurrealMemoryConfig, TelegramConfig, TunnelConfig, WebSearchConfig,
    WebhookConfig,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexported_config_default_is_constructible() {
        let config = Config::default();

        assert!(config.default_provider.is_some());
        assert!(config.default_model.is_some());
        assert!(config.default_temperature > 0.0);
    }

    #[test]
    fn reexported_channel_configs_are_constructible() {
        let telegram = TelegramConfig {
            bot_token: "token".into(),
            allowed_users: vec!["alice".into()],
            stream_mode: StreamMode::default(),
            draft_update_interval_ms: 1000,
        };

        let discord = DiscordConfig {
            bot_token: "token".into(),
            guild_id: Some("123".into()),
            allowed_users: vec![],
            listen_to_bots: false,
            mention_only: false,
        };

        let lark = LarkConfig {
            app_id: "app-id".into(),
            app_secret: "app-secret".into(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: vec![],
            use_feishu: false,
            receive_mode: crate::config::schema::LarkReceiveMode::Websocket,
            port: None,
        };

        assert_eq!(telegram.allowed_users.len(), 1);
        assert_eq!(discord.guild_id.as_deref(), Some("123"));
        assert_eq!(lark.app_id, "app-id");
    }

    #[test]
    fn reexported_plugin_configs_are_accessible() {
        let source = PluginSourceConfig {
            name: "official".to_string(),
            url: "https://example.com/catalog.json".to_string(),
            plugin_identity_regex: None,
        };
        assert_eq!(source.name, "official");
        assert!(source.url.starts_with("https://"));

        let revocation = PluginRevocationConfig {
            refresh_interval_minutes: 60,
            enabled: true,
            enforced: true,
            source_urls: vec!["https://example.com/revocations.json".to_string()],
        };
        assert!(revocation.enabled);
        assert!(revocation.enforced);
    }

    #[test]
    fn reexported_memory_config_is_constructible() {
        let memory = MemoryConfig {
            backend: "sqlite".to_string(),
            auto_save: true,
            ..Default::default()
        };
        assert_eq!(memory.backend, "sqlite");
        assert!(memory.auto_save);
    }

    #[test]
    fn reexported_autonomy_config_with_custom_limits() {
        let autonomy = AutonomyConfig {
            level: crate::security::AutonomyLevel::Supervised,
            workspace_only: true,
            allowed_commands: vec!["git".to_string(), "cargo".to_string()],
            forbidden_paths: vec!["/etc".to_string(), "/var".to_string()],
            max_actions_per_hour: 10,
            max_cost_per_day_cents: 100,
            require_approval_for_medium_risk: true,
            block_high_risk_commands: true,
            always_ask: vec![],
            auto_approve: vec![],
        };
        assert_eq!(autonomy.max_actions_per_hour, 10);
        assert_eq!(autonomy.max_cost_per_day_cents, 100);
        assert_eq!(autonomy.forbidden_paths.len(), 2);
    }

    #[test]
    fn reexported_observability_config_with_backends() {
        let obs = ObservabilityConfig {
            backend: "prometheus".to_string(),
            ..Default::default()
        };
        assert_eq!(obs.backend, "prometheus");
    }

    #[test]
    fn reexported_runtime_config_docker_settings() {
        let runtime = RuntimeConfig {
            kind: "docker".to_string(),
            docker: DockerRuntimeConfig {
                image: "custom:latest".to_string(),
                network: "bridge".to_string(),
                memory_limit_mb: Some(1024),
                cpu_limit: Some(2.0),
                read_only_rootfs: true,
                mount_workspace: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(runtime.kind, "docker");
        assert_eq!(runtime.docker.image, "custom:latest");
        assert_eq!(runtime.docker.memory_limit_mb, Some(1024));
    }

    #[test]
    fn reexported_gateway_config_defaults() {
        let gateway = GatewayConfig::default();
        assert_eq!(gateway.port, 3000);
        assert_eq!(gateway.host, "127.0.0.1");
    }

    #[test]
    fn stream_mode_variants() {
        let off = StreamMode::Off;
        let partial = StreamMode::Partial;

        assert!(matches!(off, StreamMode::Off));
        assert!(matches!(partial, StreamMode::Partial));
        assert_ne!(off, partial);
    }
}
