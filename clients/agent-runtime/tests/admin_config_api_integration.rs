use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::to_bytes,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use parking_lot::Mutex;

use corvus::{
    config::{AccountPoolStrategy, Config, ProviderAccountConfig, ProviderAccountPoolConfig},
    gateway::{admin, AppState, GatewayRateLimiter, IdempotencyStore},
    providers::Provider,
    security::pairing::PairingGuard,
};

#[derive(Default)]
struct IntegrationProvider;

#[async_trait]
impl Provider for IntegrationProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".to_string())
    }
}

fn temp_config() -> Config {
    let root = std::env::temp_dir().join(format!("corvus-admin-config-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create temp root");
    let config = Config {
        config_path: root.join("config.toml"),
        workspace_dir: root.join("workspace"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).expect("create workspace");
    config
}

fn sample_pool(account_id: &str, api_key: &str) -> ProviderAccountPoolConfig {
    ProviderAccountPoolConfig {
        strategy: AccountPoolStrategy::RoundRobin,
        accounts: vec![ProviderAccountConfig {
            id: account_id.to_string(),
            api_key: api_key.to_string(),
            api_url: None,
            weight: 1,
            enabled: true,
        }],
    }
}

fn headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("origin", HeaderValue::from_static("http://127.0.0.1:4321"));
    headers.insert(
        "authorization",
        HeaderValue::from_static("Bearer valid-token"),
    );
    headers
}

fn state_with_config(config: Config) -> AppState {
    AppState {
        config: Arc::new(Mutex::new(config)),
        cost_tracker: None,
        provider: Arc::new(IntegrationProvider),
        model: "model".into(),
        temperature: 0.7,
        mem: Arc::new(corvus::memory::NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(true, &["valid-token".into()])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 1000)),
        idempotency_store: Arc::new(IdempotencyStore::new(
            std::time::Duration::from_secs(60),
            1000,
        )),
        whatsapp: None,
        whatsapp_app_secret: None,
        channel_runtime_handle: None,
        observer: Arc::new(corvus::observability::NoopObserver),
        transcriber: None,
        audio_config: corvus::config::AudioConfig::default(),
    }
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body bytes");
    serde_json::from_slice(&bytes).expect("valid json")
}

#[tokio::test]
async fn get_admin_config_redacts_secrets() {
    let mut config = temp_config();
    config.api_key = Some("top-secret".into());
    config.channels_config.webhook = Some(corvus::config::WebhookConfig {
        port: 3000,
        secret: Some("secret".into()),
    });
    let state = state_with_config(config);

    let response = admin::handle_admin_get_config(State(state), headers())
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = body_json(response).await;
    assert_eq!(
        body.pointer("/config/provider/has_api_key"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        body.pointer("/config/channels/webhook/has_secret"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        body.pointer("/config/updates/auto_install_enabled"),
        Some(&serde_json::json!(false))
    );
    assert!(body
        .pointer("/config/updates/status/current_version")
        .is_some());
    assert!(body
        .pointer("/config/updates/status/last_check_outcome")
        .is_some());
    assert!(body
        .pointer("/config/updates/status/last_check_at_unix")
        .is_some());
    let text = body.to_string();
    assert!(!text.contains("top-secret"));
}

#[tokio::test]
async fn put_admin_config_updates_and_persists() {
    let config = temp_config();
    let state = state_with_config(config);
    let patch = admin::AdminConfigUpdateRequest {
        default_provider: None,
        default_model: None,
        api_url: None,
        default_temperature: None,
        memory_backend: None,
        provider: None,
        observability: None,
        runtime: None,
        autonomy: None,
        identity: None,
        scheduler: None,
        gateway: None,
        channels: Some(admin::AdminChannelsPatch {
            cli: Some(false),
            webhook: None,
        }),
        webhook: None,
        composio: None,
        web_search: None,
        browser: None,
        memory: None,
        cost: None,
    };

    let response =
        admin::handle_admin_update_config(State(state.clone()), headers(), Ok(Json(patch)))
            .await
            .into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = body_json(response).await;
    assert_eq!(
        body.pointer("/config/channels/cli"),
        Some(&serde_json::json!(false))
    );
    assert!(state.config.lock().config_path.exists());
}

#[tokio::test]
async fn put_admin_config_rolls_back_on_save_failure() {
    let mut config = temp_config();
    config.config_path = std::path::PathBuf::from("/nonexistent/corvus/config.toml");
    let state = state_with_config(config);
    let patch = admin::AdminConfigUpdateRequest {
        default_provider: Some("openrouter".into()),
        default_model: None,
        api_url: None,
        default_temperature: None,
        memory_backend: None,
        provider: None,
        observability: None,
        runtime: None,
        autonomy: None,
        identity: None,
        scheduler: None,
        gateway: None,
        channels: None,
        webhook: None,
        composio: None,
        web_search: None,
        browser: None,
        memory: None,
        cost: None,
    };

    let before = state.config.lock().default_provider.clone();
    let response =
        admin::handle_admin_update_config(State(state.clone()), headers(), Ok(Json(patch)))
            .await
            .into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(state.config.lock().default_provider, before);
}

#[tokio::test]
async fn admin_provider_pools_rejects_when_disabled() {
    let mut config = temp_config();
    config
        .reliability
        .account_pools
        .insert("openrouter".into(), sample_pool("acct-a", "secret-key"));
    let state = state_with_config(config);

    let response = admin::handle_admin_get_provider_pools(State(state.clone()), headers())
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let patch = admin::AdminProviderPoolsPatch {
        account_pools: std::collections::HashMap::new(),
    };
    let response =
        admin::handle_admin_update_provider_pools(State(state), headers(), Ok(Json(patch)))
            .await
            .into_response();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_provider_pools_redacts_api_keys() {
    let mut config = temp_config();
    config.gateway.admin_expose_provider_pools = true;
    config
        .reliability
        .account_pools
        .insert("openrouter".into(), sample_pool("acct-a", "secret-key"));
    let state = state_with_config(config);

    let response = admin::handle_admin_get_provider_pools(State(state), headers())
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = body_json(response).await;
    assert_eq!(
        body.pointer("/pools/account_pools/openrouter/accounts/0/has_api_key"),
        Some(&serde_json::json!(true))
    );
    let text = body.to_string();
    assert!(!text.contains("secret-key"));
}

#[tokio::test]
async fn admin_provider_pools_rejects_invalid_patch_when_enabled() {
    let mut config = temp_config();
    config.gateway.admin_expose_provider_pools = true;
    config
        .reliability
        .account_pools
        .insert("openrouter".into(), sample_pool("acct-a", "secret-key"));
    let state = state_with_config(config);

    let patch = admin::AdminProviderPoolsPatch {
        account_pools: std::collections::HashMap::from([(
            "openrouter".into(),
            ProviderAccountPoolConfig {
                strategy: AccountPoolStrategy::RoundRobin,
                accounts: vec![ProviderAccountConfig {
                    id: "".into(),
                    api_key: "missing".into(),
                    api_url: None,
                    weight: 1,
                    enabled: true,
                }],
            },
        )]),
    };

    let response =
        admin::handle_admin_update_provider_pools(State(state.clone()), headers(), Ok(Json(patch)))
            .await
            .into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let stored = state.config.lock();
    let pool = stored.reliability.account_pools.get("openrouter").unwrap();
    assert_eq!(pool.accounts[0].id, "acct-a");
}
