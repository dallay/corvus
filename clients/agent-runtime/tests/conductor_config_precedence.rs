use corvus::conductor::config::resolve_tick_interval_ms;
use corvus::config::ConductorConfig;

#[test]
fn conductor_markdown_front_matter_overrides_config_toml_tick_interval() {
    let config = ConductorConfig {
        tick_interval_ms: 30_000,
        ..ConductorConfig::default()
    };

    let markdown = r#"---
tick_interval_ms: 15000
---
# Conductor
"#;

    let resolved = resolve_tick_interval_ms(&config, Some(markdown));
    assert_eq!(resolved, 15_000);
}

#[test]
fn config_toml_tick_interval_used_when_front_matter_missing_or_invalid() {
    let config = ConductorConfig {
        tick_interval_ms: 30_000,
        ..ConductorConfig::default()
    };

    let without_front_matter = "# no front matter";
    let invalid_front_matter = r#"---
tick_interval_ms: nope
---
"#;

    assert_eq!(
        resolve_tick_interval_ms(&config, Some(without_front_matter)),
        30_000,
    );
    assert_eq!(
        resolve_tick_interval_ms(&config, Some(invalid_front_matter)),
        30_000,
    );
}
