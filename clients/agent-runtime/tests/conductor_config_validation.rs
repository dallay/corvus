use corvus::config::{ConcurrencyConfig, ConductorConfig, Config, PerformerConfig};

#[test]
fn default_configuration_keeps_conductor_inactive() {
    let config = Config::default();

    assert!(!config.conductor.enabled);
}

#[test]
fn invalid_conductor_timeouts_are_rejected() {
    let config = Config {
        conductor: ConductorConfig {
            enabled: true,
            tick_interval_ms: 0,
            ..ConductorConfig::default()
        },
        ..Config::default()
    };

    let error = config
        .validate_for_runtime()
        .expect_err("expected invalid config");
    assert!(
        error.to_string().contains("conductor.tick_interval_ms"),
        "error should point to conductor.tick_interval_ms: {error}",
    );
}

#[test]
fn invalid_conductor_concurrency_is_rejected() {
    let config = Config {
        conductor: ConductorConfig {
            enabled: true,
            concurrency: ConcurrencyConfig {
                global_max: 1,
                coding_max: 0,
                ..ConcurrencyConfig::default()
            },
            ..ConductorConfig::default()
        },
        ..Config::default()
    };

    let error = config
        .validate_for_runtime()
        .expect_err("expected invalid config");
    assert!(
        error
            .to_string()
            .contains("conductor.concurrency.coding_max"),
        "error should point to conductor.concurrency.coding_max: {error}",
    );
}

#[test]
fn system_performer_requires_approval_by_default() {
    let config = Config::default();

    assert!(config.conductor.performers.system.approval_required);
}

#[test]
fn unsafe_system_performer_policy_is_rejected() {
    let config = Config {
        conductor: ConductorConfig {
            enabled: true,
            performers: corvus::config::PerformerConfigs {
                system: PerformerConfig {
                    approval_required: false,
                    ..PerformerConfig::default()
                },
                ..corvus::config::PerformerConfigs::default()
            },
            ..ConductorConfig::default()
        },
        ..Config::default()
    };

    let error = config
        .validate_for_runtime()
        .expect_err("expected invalid config");
    assert!(
        error
            .to_string()
            .contains("conductor.performers.system.approval_required"),
        "error should point to conductor.performers.system.approval_required: {error}",
    );
}
