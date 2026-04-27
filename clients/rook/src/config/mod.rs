//! Config — Rook-specific configuration loading and validation.
//!
//! Owns the `RookConfig` struct and its TOML/env loading logic. Intentionally
//! separate from the `corvus` binary's `Config` type — Rook has its own
//! independent configuration schema and file path (`~/.config/rook/config.toml`
//! by default).
//!
//! FIXME: implement `RookConfig` struct with gateway, registry, and TUI sections.
//! FIXME: add env-var overrides (`ROOK_*` prefix).
//! FIXME: add `rook config validate` to the CLI once struct is stable.

use crate::domain::RookError;
use crate::server::ServerConfig;
use axum::http::HeaderName;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub fn discover_default_config_path(env_map: &HashMap<String, String>) -> Option<PathBuf> {
    if let Some(xdg_config_home) = env_map.get("XDG_CONFIG_HOME") {
        return Some(
            PathBuf::from(xdg_config_home)
                .join("rook")
                .join("config.toml"),
        );
    }

    env_map.get("HOME").map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join("rook")
            .join("config.toml")
    })
}

pub fn discover_default_config_path_from_env() -> Option<PathBuf> {
    let env_map = env::vars().collect::<HashMap<String, String>>();
    discover_default_config_path(&env_map)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RookConfig {
    pub host: String,
    pub port: u16,
    pub enable_tui: bool,
    pub db_path: PathBuf,
    pub inbound_auth: InboundAuthConfig,
    pub transport: TransportConfig,
    pub rate_limits: RateLimitConfig,
    pub idempotency: IdempotencyConfig,
}

impl Default for RookConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 4141,
            enable_tui: false,
            db_path: PathBuf::from("./rook.db"),
            inbound_auth: InboundAuthConfig::default(),
            transport: TransportConfig::default(),
            rate_limits: RateLimitConfig::default(),
            idempotency: IdempotencyConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartialRookConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub enable_tui: Option<bool>,
    pub db_path: Option<PathBuf>,
    pub inbound_auth: Option<PartialInboundAuthConfig>,
    pub transport: Option<PartialTransportConfig>,
    pub rate_limits: Option<PartialRateLimitConfig>,
    pub idempotency: Option<PartialIdempotencyConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartialInboundAuthConfig {
    pub enabled: Option<bool>,
    pub bearer_token: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartialTransportConfig {
    pub request_id: Option<PartialRequestIdConfig>,
    pub trusted_proxy: Option<PartialTrustedProxyConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartialRequestIdConfig {
    pub inbound_header_name: Option<String>,
    pub response_header_name: Option<String>,
    pub max_length: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartialTrustedProxyConfig {
    pub enabled: Option<bool>,
    pub trusted_cidrs: Option<Vec<String>>,
    pub allowed_headers: Option<PartialTrustedForwardedHeaders>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartialTrustedForwardedHeaders {
    pub forwarded: Option<bool>,
    pub x_forwarded_for: Option<bool>,
    pub x_forwarded_host: Option<bool>,
    pub x_forwarded_proto: Option<bool>,
    pub x_forwarded_port: Option<bool>,
    pub x_real_ip: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartialRateLimitConfig {
    pub api: Option<PartialSurfaceRateLimitPolicy>,
    pub v1_models: Option<PartialSurfaceRateLimitPolicy>,
    pub v1_chat_completions: Option<PartialSurfaceRateLimitPolicy>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartialSurfaceRateLimitPolicy {
    pub max_requests: Option<u32>,
    pub window_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartialIdempotencyConfig {
    pub chat_completions: Option<PartialChatCompletionsIdempotencyConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartialChatCompletionsIdempotencyConfig {
    pub enabled: Option<bool>,
    pub replay_window_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct CliRookConfigOverlay {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub enable_tui: Option<bool>,
    pub db_path: Option<PathBuf>,
    pub inbound_auth: Option<PartialInboundAuthConfig>,
    pub transport: Option<PartialTransportConfig>,
    pub rate_limits: Option<PartialRateLimitConfig>,
    pub idempotency: Option<PartialIdempotencyConfig>,
}

pub struct LoadRookConfigInput<'a> {
    pub file_path: Option<&'a Path>,
    pub env: &'a HashMap<String, String>,
    pub cli: Option<CliRookConfigOverlay>,
}

impl RookConfig {
    pub fn load_from_file(path: &Path) -> Result<Self, RookError> {
        let content = fs::read_to_string(path).map_err(RookError::Io)?;
        Self::from_toml_str(&content)
    }

    pub fn load_from_optional_file(path: Option<&Path>) -> Result<Self, RookError> {
        match path {
            Some(path) if path.exists() => Self::load_from_file(path),
            Some(_) | None => Ok(Self::default()),
        }
    }

    pub fn from_toml_str(input: &str) -> Result<Self, RookError> {
        let partial: PartialRookConfig = toml::from_str(input)
            .map_err(|error| RookError::Config(format!("invalid rook config TOML: {error}")))?;

        let mut config = Self::default();
        partial.apply_to(&mut config);
        Ok(config)
    }

    pub fn from_sources(
        file_toml: Option<&str>,
        env: &HashMap<String, String>,
    ) -> Result<Self, RookError> {
        let mut config = Self::default();

        if let Some(file_toml) = file_toml {
            let partial: PartialRookConfig = toml::from_str(file_toml)
                .map_err(|error| RookError::Config(format!("invalid rook config TOML: {error}")))?;
            partial.apply_to(&mut config);
        }

        parse_env_overlay(env)?.apply_to(&mut config);
        config.validate()?;
        Ok(config)
    }

    pub fn from_sources_with_path(
        file_path: Option<&Path>,
        env: &HashMap<String, String>,
    ) -> Result<Self, RookError> {
        load_effective_config(LoadRookConfigInput {
            file_path,
            env,
            cli: None,
        })
    }

    pub fn apply_env_overrides(&mut self, env: &HashMap<String, String>) -> Result<(), RookError> {
        parse_env_overlay(env)?.apply_to(self);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), RookError> {
        if self.host.trim().is_empty() {
            return Err(RookError::Config(
                "server host must not be blank".to_string(),
            ));
        }

        if self.db_path.as_os_str().is_empty() {
            return Err(RookError::Config(
                "database path must not be blank".to_string(),
            ));
        }

        self.inbound_auth.validate()?;
        self.transport.validate()?;
        self.rate_limits.validate()?;
        self.idempotency.validate()?;
        Ok(())
    }

    pub fn to_server_config(&self) -> ServerConfig {
        ServerConfig {
            host: self.host.clone(),
            port: self.port,
            enable_tui: self.enable_tui,
            db_path: Some(self.db_path.display().to_string()),
            inbound_auth: self.inbound_auth.clone(),
            transport: self.transport.clone(),
            rate_limits: self.rate_limits.clone(),
            idempotency: self.idempotency.clone(),
        }
    }
}

impl PartialRookConfig {
    fn apply_to(self, target: &mut RookConfig) {
        if let Some(host) = self.host {
            target.host = host;
        }
        if let Some(port) = self.port {
            target.port = port;
        }
        if let Some(enable_tui) = self.enable_tui {
            target.enable_tui = enable_tui;
        }
        if let Some(db_path) = self.db_path {
            target.db_path = db_path;
        }
        if let Some(inbound_auth) = self.inbound_auth {
            inbound_auth.apply_to(&mut target.inbound_auth);
        }
        if let Some(transport) = self.transport {
            transport.apply_to(&mut target.transport);
        }
        if let Some(rate_limits) = self.rate_limits {
            rate_limits.apply_to(&mut target.rate_limits);
        }
        if let Some(idempotency) = self.idempotency {
            idempotency.apply_to(&mut target.idempotency);
        }
    }
}

impl PartialInboundAuthConfig {
    fn apply_to(self, target: &mut InboundAuthConfig) {
        if let Some(enabled) = self.enabled {
            target.enabled = enabled;
        }
        if let Some(bearer_token) = self.bearer_token {
            target.bearer_token = Some(bearer_token);
        }
    }
}

impl PartialTransportConfig {
    fn apply_to(self, target: &mut TransportConfig) {
        if let Some(request_id) = self.request_id {
            request_id.apply_to(&mut target.request_id);
        }
        if let Some(trusted_proxy) = self.trusted_proxy {
            trusted_proxy.apply_to(&mut target.trusted_proxy);
        }
    }
}

impl PartialRequestIdConfig {
    fn apply_to(self, target: &mut RequestIdConfig) {
        if let Some(inbound_header_name) = self.inbound_header_name {
            target.inbound_header_name = inbound_header_name;
        }
        if let Some(response_header_name) = self.response_header_name {
            target.response_header_name = response_header_name;
        }
        if let Some(max_length) = self.max_length {
            target.max_length = max_length;
        }
    }
}

impl PartialTrustedProxyConfig {
    fn apply_to(self, target: &mut TrustedProxyConfig) {
        if let Some(enabled) = self.enabled {
            target.enabled = enabled;
        }
        if let Some(trusted_cidrs) = self.trusted_cidrs {
            target.trusted_cidrs = trusted_cidrs;
        }
        if let Some(allowed_headers) = self.allowed_headers {
            allowed_headers.apply_to(&mut target.allowed_headers);
        }
    }
}

impl PartialTrustedForwardedHeaders {
    fn apply_to(self, target: &mut TrustedForwardedHeaders) {
        if let Some(forwarded) = self.forwarded {
            target.forwarded = forwarded;
        }
        if let Some(x_forwarded_for) = self.x_forwarded_for {
            target.x_forwarded_for = x_forwarded_for;
        }
        if let Some(x_forwarded_host) = self.x_forwarded_host {
            target.x_forwarded_host = x_forwarded_host;
        }
        if let Some(x_forwarded_proto) = self.x_forwarded_proto {
            target.x_forwarded_proto = x_forwarded_proto;
        }
        if let Some(x_forwarded_port) = self.x_forwarded_port {
            target.x_forwarded_port = x_forwarded_port;
        }
        if let Some(x_real_ip) = self.x_real_ip {
            target.x_real_ip = x_real_ip;
        }
    }
}

impl PartialRateLimitConfig {
    fn apply_to(self, target: &mut RateLimitConfig) {
        if let Some(api) = self.api {
            api.apply_to(&mut target.api);
        }
        if let Some(v1_models) = self.v1_models {
            v1_models.apply_to(&mut target.v1_models);
        }
        if let Some(v1_chat_completions) = self.v1_chat_completions {
            v1_chat_completions.apply_to(&mut target.v1_chat_completions);
        }
    }
}

impl PartialSurfaceRateLimitPolicy {
    fn apply_to(self, target: &mut SurfaceRateLimitPolicy) {
        if let Some(max_requests) = self.max_requests {
            target.max_requests = max_requests;
        }
        if let Some(window_seconds) = self.window_seconds {
            target.window_seconds = window_seconds;
        }
    }
}

impl PartialIdempotencyConfig {
    fn apply_to(self, target: &mut IdempotencyConfig) {
        if let Some(chat_completions) = self.chat_completions {
            chat_completions.apply_to(&mut target.chat_completions);
        }
    }
}

impl PartialChatCompletionsIdempotencyConfig {
    fn apply_to(self, target: &mut ChatCompletionsIdempotencyConfig) {
        if let Some(enabled) = self.enabled {
            target.enabled = enabled;
        }
        if let Some(replay_window_seconds) = self.replay_window_seconds {
            target.replay_window_seconds = replay_window_seconds;
        }
    }
}

impl CliRookConfigOverlay {
    fn apply_to(self, target: &mut RookConfig) {
        PartialRookConfig {
            host: self.host,
            port: self.port,
            enable_tui: self.enable_tui,
            db_path: self.db_path,
            inbound_auth: self.inbound_auth,
            transport: self.transport,
            rate_limits: self.rate_limits,
            idempotency: self.idempotency,
        }
        .apply_to(target);
    }
}

fn load_file_overlay(file_path: Option<&Path>) -> Result<PartialRookConfig, RookError> {
    match file_path {
        Some(path) if path.exists() => {
            let content = fs::read_to_string(path).map_err(RookError::Io)?;
            toml::from_str(&content)
                .map_err(|error| RookError::Config(format!("invalid rook config TOML: {error}")))
        }
        Some(_) | None => Ok(PartialRookConfig::default()),
    }
}

fn parse_env_overlay(env: &HashMap<String, String>) -> Result<PartialRookConfig, RookError> {
    Ok(PartialRookConfig {
        host: env.get("ROOK_HOST").cloned(),
        port: override_from_env::<u16>(env, "ROOK_PORT")?,
        enable_tui: env
            .get("ROOK_ENABLE_TUI")
            .map(|value| parse_bool_env("ROOK_ENABLE_TUI", value))
            .transpose()?,
        db_path: env.get("ROOK_DB_PATH").map(PathBuf::from),
        inbound_auth: partial_if_any(PartialInboundAuthConfig {
            enabled: env
                .get("ROOK_INBOUND_AUTH_ENABLED")
                .map(|value| parse_bool_env("ROOK_INBOUND_AUTH_ENABLED", value))
                .transpose()?,
            bearer_token: env.get("ROOK_INBOUND_AUTH_TOKEN").cloned(),
        }),
        transport: partial_if_any(PartialTransportConfig {
            request_id: partial_if_any(PartialRequestIdConfig {
                inbound_header_name: env
                    .get("ROOK_TRANSPORT_REQUEST_ID_INBOUND_HEADER_NAME")
                    .cloned(),
                response_header_name: env
                    .get("ROOK_TRANSPORT_REQUEST_ID_RESPONSE_HEADER_NAME")
                    .cloned(),
                max_length: override_from_env::<usize>(
                    env,
                    "ROOK_TRANSPORT_REQUEST_ID_MAX_LENGTH",
                )?,
            }),
            trusted_proxy: partial_if_any(PartialTrustedProxyConfig {
                enabled: env
                    .get("ROOK_TRANSPORT_TRUSTED_PROXY_ENABLED")
                    .map(|value| parse_bool_env("ROOK_TRANSPORT_TRUSTED_PROXY_ENABLED", value))
                    .transpose()?,
                trusted_cidrs: env.get("ROOK_TRANSPORT_TRUSTED_PROXY_TRUSTED_CIDRS").map(
                    |trusted_cidrs| {
                        trusted_cidrs
                            .split(',')
                            .map(str::trim)
                            .filter(|cidr| !cidr.is_empty())
                            .map(ToOwned::to_owned)
                            .collect()
                    },
                ),
                allowed_headers: partial_if_any(PartialTrustedForwardedHeaders {
                    forwarded: env
                        .get("ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_FORWARDED")
                        .map(|value| {
                            parse_bool_env("ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_FORWARDED", value)
                        })
                        .transpose()?,
                    x_forwarded_for: env
                        .get("ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_FOR")
                        .map(|value| {
                            parse_bool_env(
                                "ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_FOR",
                                value,
                            )
                        })
                        .transpose()?,
                    x_forwarded_host: env
                        .get("ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_HOST")
                        .map(|value| {
                            parse_bool_env(
                                "ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_HOST",
                                value,
                            )
                        })
                        .transpose()?,
                    x_forwarded_proto: env
                        .get("ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_PROTO")
                        .map(|value| {
                            parse_bool_env(
                                "ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_PROTO",
                                value,
                            )
                        })
                        .transpose()?,
                    x_forwarded_port: env
                        .get("ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_PORT")
                        .map(|value| {
                            parse_bool_env(
                                "ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_PORT",
                                value,
                            )
                        })
                        .transpose()?,
                    x_real_ip: env
                        .get("ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_REAL_IP")
                        .map(|value| {
                            parse_bool_env("ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_REAL_IP", value)
                        })
                        .transpose()?,
                }),
            }),
        }),
        rate_limits: partial_if_any(PartialRateLimitConfig {
            api: partial_if_any(PartialSurfaceRateLimitPolicy {
                max_requests: override_from_env::<u32>(env, "ROOK_API_RATE_LIMIT_MAX_REQUESTS")?,
                window_seconds: override_from_env::<u64>(
                    env,
                    "ROOK_API_RATE_LIMIT_WINDOW_SECONDS",
                )?,
            }),
            v1_models: partial_if_any(PartialSurfaceRateLimitPolicy {
                max_requests: override_from_env::<u32>(
                    env,
                    "ROOK_V1_MODELS_RATE_LIMIT_MAX_REQUESTS",
                )?,
                window_seconds: override_from_env::<u64>(
                    env,
                    "ROOK_V1_MODELS_RATE_LIMIT_WINDOW_SECONDS",
                )?,
            }),
            v1_chat_completions: partial_if_any(PartialSurfaceRateLimitPolicy {
                max_requests: override_from_env::<u32>(
                    env,
                    "ROOK_V1_CHAT_RATE_LIMIT_MAX_REQUESTS",
                )?,
                window_seconds: override_from_env::<u64>(
                    env,
                    "ROOK_V1_CHAT_RATE_LIMIT_WINDOW_SECONDS",
                )?,
            }),
        }),
        idempotency: partial_if_any(PartialIdempotencyConfig {
            chat_completions: partial_if_any(PartialChatCompletionsIdempotencyConfig {
                enabled: env
                    .get("ROOK_CHAT_IDEMPOTENCY_ENABLED")
                    .map(|value| parse_bool_env("ROOK_CHAT_IDEMPOTENCY_ENABLED", value))
                    .transpose()?,
                replay_window_seconds: override_from_env::<u64>(
                    env,
                    "ROOK_CHAT_IDEMPOTENCY_REPLAY_WINDOW_SECONDS",
                )?,
            }),
        }),
    })
}

pub fn load_effective_config(input: LoadRookConfigInput<'_>) -> Result<RookConfig, RookError> {
    let mut config = RookConfig::default();
    load_file_overlay(input.file_path)?.apply_to(&mut config);
    parse_env_overlay(input.env)?.apply_to(&mut config);
    if let Some(cli) = input.cli {
        cli.apply_to(&mut config);
    }
    config.validate()?;
    Ok(config)
}

trait PartialOverlay {
    fn is_empty(&self) -> bool;
}

impl PartialOverlay for PartialInboundAuthConfig {
    fn is_empty(&self) -> bool {
        self.enabled.is_none() && self.bearer_token.is_none()
    }
}

impl PartialOverlay for PartialRequestIdConfig {
    fn is_empty(&self) -> bool {
        self.inbound_header_name.is_none()
            && self.response_header_name.is_none()
            && self.max_length.is_none()
    }
}

impl PartialOverlay for PartialTrustedForwardedHeaders {
    fn is_empty(&self) -> bool {
        self.forwarded.is_none()
            && self.x_forwarded_for.is_none()
            && self.x_forwarded_host.is_none()
            && self.x_forwarded_proto.is_none()
            && self.x_forwarded_port.is_none()
            && self.x_real_ip.is_none()
    }
}

impl PartialOverlay for PartialTrustedProxyConfig {
    fn is_empty(&self) -> bool {
        self.enabled.is_none() && self.trusted_cidrs.is_none() && self.allowed_headers.is_none()
    }
}

impl PartialOverlay for PartialTransportConfig {
    fn is_empty(&self) -> bool {
        self.request_id.is_none() && self.trusted_proxy.is_none()
    }
}

impl PartialOverlay for PartialSurfaceRateLimitPolicy {
    fn is_empty(&self) -> bool {
        self.max_requests.is_none() && self.window_seconds.is_none()
    }
}

impl PartialOverlay for PartialRateLimitConfig {
    fn is_empty(&self) -> bool {
        self.api.is_none() && self.v1_models.is_none() && self.v1_chat_completions.is_none()
    }
}

impl PartialOverlay for PartialChatCompletionsIdempotencyConfig {
    fn is_empty(&self) -> bool {
        self.enabled.is_none() && self.replay_window_seconds.is_none()
    }
}

impl PartialOverlay for PartialIdempotencyConfig {
    fn is_empty(&self) -> bool {
        self.chat_completions.is_none()
    }
}

fn partial_if_any<T: PartialOverlay>(partial: T) -> Option<T> {
    if partial.is_empty() {
        None
    } else {
        Some(partial)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RookConfigExportView {
    pub host: String,
    pub port: u16,
    pub enable_tui: bool,
    pub db_path: String,
    pub inbound_auth: InboundAuthExportView,
    pub transport: TransportExportView,
    pub rate_limits: RateLimitExportView,
    pub idempotency: IdempotencyExportView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InboundAuthExportView {
    pub enabled: bool,
    pub bearer_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransportExportView {
    pub request_id: RequestIdExportView,
    pub trusted_proxy: TrustedProxyExportView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequestIdExportView {
    pub inbound_header_name: String,
    pub response_header_name: String,
    pub max_length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TrustedProxyExportView {
    pub enabled: bool,
    pub trusted_cidrs: Vec<String>,
    pub allowed_headers: TrustedForwardedHeadersExportView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TrustedForwardedHeadersExportView {
    pub forwarded: bool,
    pub x_forwarded_for: bool,
    pub x_forwarded_host: bool,
    pub x_forwarded_proto: bool,
    pub x_forwarded_port: bool,
    pub x_real_ip: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RateLimitExportView {
    pub api: SurfaceRateLimitPolicy,
    pub v1_models: SurfaceRateLimitPolicy,
    pub v1_chat_completions: SurfaceRateLimitPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IdempotencyExportView {
    pub chat_completions: ChatCompletionsIdempotencyExportView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChatCompletionsIdempotencyExportView {
    pub enabled: bool,
    pub replay_window_seconds: u64,
}

fn parse_numeric_env<T: FromStr>(name: &str, value: &str) -> Result<T, RookError>
where
    T::Err: std::fmt::Display,
{
    value
        .parse::<T>()
        .map_err(|error| RookError::Config(format!("invalid {name} value '{value}': {error}")))
}

fn override_from_env<T: FromStr>(
    env: &HashMap<String, String>,
    name: &str,
) -> Result<Option<T>, RookError>
where
    T::Err: std::fmt::Display,
{
    env.get(name)
        .map(|value| parse_numeric_env(name, value))
        .transpose()
}

fn parse_bool_env(name: &str, value: &str) -> Result<bool, RookError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(RookError::Config(format!("invalid {name} value '{value}'"))),
    }
}

fn redact_optional_secret(secret: Option<&str>) -> String {
    match secret {
        Some(value) if !value.trim().is_empty() => "[redacted]".to_string(),
        _ => "[not configured]".to_string(),
    }
}

impl RookConfigExportView {
    pub fn from_config(config: &RookConfig) -> Self {
        Self {
            host: config.host.clone(),
            port: config.port,
            enable_tui: config.enable_tui,
            db_path: config.db_path.display().to_string(),
            inbound_auth: InboundAuthExportView {
                enabled: config.inbound_auth.enabled,
                bearer_token: if config.inbound_auth.enabled {
                    Some(redact_optional_secret(
                        config.inbound_auth.bearer_token.as_deref(),
                    ))
                } else {
                    None
                },
            },
            transport: TransportExportView {
                request_id: RequestIdExportView {
                    inbound_header_name: config.transport.request_id.inbound_header_name.clone(),
                    response_header_name: config.transport.request_id.response_header_name.clone(),
                    max_length: config.transport.request_id.max_length,
                },
                trusted_proxy: TrustedProxyExportView {
                    enabled: config.transport.trusted_proxy.enabled,
                    trusted_cidrs: config.transport.trusted_proxy.trusted_cidrs.clone(),
                    allowed_headers: TrustedForwardedHeadersExportView {
                        forwarded: config.transport.trusted_proxy.allowed_headers.forwarded,
                        x_forwarded_for: config
                            .transport
                            .trusted_proxy
                            .allowed_headers
                            .x_forwarded_for,
                        x_forwarded_host: config
                            .transport
                            .trusted_proxy
                            .allowed_headers
                            .x_forwarded_host,
                        x_forwarded_proto: config
                            .transport
                            .trusted_proxy
                            .allowed_headers
                            .x_forwarded_proto,
                        x_forwarded_port: config
                            .transport
                            .trusted_proxy
                            .allowed_headers
                            .x_forwarded_port,
                        x_real_ip: config.transport.trusted_proxy.allowed_headers.x_real_ip,
                    },
                },
            },
            rate_limits: RateLimitExportView {
                api: config.rate_limits.api.clone(),
                v1_models: config.rate_limits.v1_models.clone(),
                v1_chat_completions: config.rate_limits.v1_chat_completions.clone(),
            },
            idempotency: IdempotencyExportView {
                chat_completions: ChatCompletionsIdempotencyExportView {
                    enabled: config.idempotency.chat_completions.enabled,
                    replay_window_seconds: config
                        .idempotency
                        .chat_completions
                        .replay_window_seconds,
                },
            },
        }
    }
}

#[derive(Clone, PartialEq, Eq, Default)]
pub struct InboundAuthConfig {
    pub enabled: bool,
    pub bearer_token: Option<String>,
}

impl std::fmt::Debug for InboundAuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InboundAuthConfig")
            .field("enabled", &self.enabled)
            .field(
                "bearer_token",
                &self.bearer_token.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboundAuthOperatorState {
    pub enabled: bool,
    pub token_configured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatCompletionsIdempotencyConfig {
    pub enabled: bool,
    pub replay_window_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IdempotencyConfig {
    pub chat_completions: ChatCompletionsIdempotencyConfig,
}

impl Default for ChatCompletionsIdempotencyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            replay_window_seconds: 86_400,
        }
    }
}

impl ChatCompletionsIdempotencyConfig {
    pub fn validate(&self) -> Result<(), RookError> {
        if self.replay_window_seconds == 0 {
            return Err(RookError::Config(
                "chat completions idempotency replay window must be greater than zero".to_string(),
            ));
        }

        Ok(())
    }
}

impl IdempotencyConfig {
    pub fn validate(&self) -> Result<(), RookError> {
        self.chat_completions.validate()
    }
}

impl InboundAuthConfig {
    pub fn token_configured(&self) -> bool {
        self.bearer_token
            .as_deref()
            .map(str::trim)
            .is_some_and(|token| !token.is_empty())
    }

    pub fn operator_state(&self) -> InboundAuthOperatorState {
        InboundAuthOperatorState {
            enabled: self.enabled,
            token_configured: self.token_configured(),
        }
    }

    pub fn validate(&self) -> Result<(), RookError> {
        if !self.enabled {
            return Ok(());
        }

        let token = self.bearer_token.as_deref().ok_or_else(|| {
            RookError::Config("inbound auth token is required when auth is enabled".to_string())
        })?;

        if token.trim().is_empty() {
            return Err(RookError::Config(
                "inbound auth token must not be blank when auth is enabled".to_string(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestIdConfig {
    pub inbound_header_name: String,
    pub response_header_name: String,
    pub max_length: usize,
}

impl Default for RequestIdConfig {
    fn default() -> Self {
        Self {
            inbound_header_name: "x-request-id".to_string(),
            response_header_name: "x-request-id".to_string(),
            max_length: 128,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrustedForwardedHeaders {
    pub forwarded: bool,
    pub x_forwarded_for: bool,
    pub x_forwarded_host: bool,
    pub x_forwarded_proto: bool,
    pub x_forwarded_port: bool,
    pub x_real_ip: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrustedProxyConfig {
    pub enabled: bool,
    pub trusted_cidrs: Vec<String>,
    pub allowed_headers: TrustedForwardedHeaders,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransportConfig {
    pub request_id: RequestIdConfig,
    pub trusted_proxy: TrustedProxyConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceRateLimitPolicy {
    pub max_requests: u32,
    pub window_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimitConfig {
    pub api: SurfaceRateLimitPolicy,
    pub v1_models: SurfaceRateLimitPolicy,
    pub v1_chat_completions: SurfaceRateLimitPolicy,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            api: SurfaceRateLimitPolicy {
                max_requests: 60,
                window_seconds: 60,
            },
            v1_models: SurfaceRateLimitPolicy {
                max_requests: 120,
                window_seconds: 60,
            },
            v1_chat_completions: SurfaceRateLimitPolicy {
                max_requests: 30,
                window_seconds: 60,
            },
        }
    }
}

impl RateLimitConfig {
    pub fn validate(&self) -> Result<(), RookError> {
        self.api.validate("/api/*")?;
        self.v1_models.validate("/v1/models")?;
        self.v1_chat_completions.validate("/v1/chat/completions")?;
        Ok(())
    }
}

impl SurfaceRateLimitPolicy {
    fn validate(&self, surface: &str) -> Result<(), RookError> {
        if self.max_requests == 0 {
            return Err(RookError::Config(format!(
                "rate limit max_requests for {surface} must be greater than zero"
            )));
        }

        if self.window_seconds == 0 {
            return Err(RookError::Config(format!(
                "rate limit window_seconds for {surface} must be greater than zero"
            )));
        }

        Ok(())
    }
}

impl TransportConfig {
    pub fn validate(&self) -> Result<(), RookError> {
        if self.request_id.inbound_header_name.trim().is_empty() {
            return Err(RookError::Config(
                "request ID inbound header name must not be blank".to_string(),
            ));
        }

        HeaderName::from_str(&self.request_id.inbound_header_name).map_err(|error| {
            RookError::Config(format!(
                "request ID inbound header name is invalid: {error}"
            ))
        })?;

        if self.request_id.response_header_name.trim().is_empty() {
            return Err(RookError::Config(
                "request ID response header name must not be blank".to_string(),
            ));
        }

        HeaderName::from_str(&self.request_id.response_header_name).map_err(|error| {
            RookError::Config(format!(
                "request ID response header name is invalid: {error}"
            ))
        })?;

        if self.request_id.max_length == 0 {
            return Err(RookError::Config(
                "request ID max length must be greater than zero".to_string(),
            ));
        }

        if !self.trusted_proxy.enabled {
            return Ok(());
        }

        if self.trusted_proxy.trusted_cidrs.is_empty() {
            return Err(RookError::Config(
                "trusted proxy CIDR list must not be empty when trusted proxy is enabled"
                    .to_string(),
            ));
        }

        for cidr in &self.trusted_proxy.trusted_cidrs {
            IpNet::from_str(cidr).map_err(|error| {
                RookError::Config(format!("invalid trusted proxy CIDR '{cidr}': {error}"))
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        load_effective_config, parse_env_overlay, ChatCompletionsIdempotencyConfig,
        CliRookConfigOverlay, IdempotencyConfig, InboundAuthConfig, InboundAuthOperatorState,
        LoadRookConfigInput, PartialChatCompletionsIdempotencyConfig, PartialIdempotencyConfig,
        PartialInboundAuthConfig, PartialRateLimitConfig, PartialSurfaceRateLimitPolicy,
        RateLimitConfig, RequestIdConfig, SurfaceRateLimitPolicy, TransportConfig,
        TrustedForwardedHeaders, TrustedProxyConfig,
    };
    use serde_json::json;

    #[test]
    fn rook_config_default_will_exist_for_phase_1() {
        let _ = super::RookConfig::default();
    }

    #[test]
    fn load_effective_config_applies_defaults_then_file_then_env_then_cli() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let config_path = temp_dir.path().join("rook.toml");
        std::fs::write(
            &config_path,
            r#"
            host = "1.1.1.1"
            port = 6464
            db_path = "/file/rook.db"

            [inbound_auth]
            enabled = true
            bearer_token = "file-token"

            [rate_limits.api]
            max_requests = 71
            window_seconds = 41
            "#,
        )
        .expect("config file should be written");

        let env = std::collections::HashMap::from([
            ("ROOK_HOST".to_string(), "2.2.2.2".to_string()),
            ("ROOK_PORT".to_string(), "7474".to_string()),
            ("ROOK_DB_PATH".to_string(), "/env/rook.db".to_string()),
            (
                "ROOK_INBOUND_AUTH_TOKEN".to_string(),
                "env-token".to_string(),
            ),
            (
                "ROOK_API_RATE_LIMIT_MAX_REQUESTS".to_string(),
                "81".to_string(),
            ),
        ]);

        let config = load_effective_config(LoadRookConfigInput {
            file_path: Some(config_path.as_path()),
            env: &env,
            cli: Some(CliRookConfigOverlay {
                host: Some("3.3.3.3".to_string()),
                port: Some(8484),
                enable_tui: Some(true),
                db_path: Some(std::path::PathBuf::from("/cli/rook.db")),
                inbound_auth: Some(PartialInboundAuthConfig {
                    enabled: Some(true),
                    bearer_token: Some("cli-token".to_string()),
                }),
                transport: None,
                rate_limits: Some(PartialRateLimitConfig {
                    api: Some(PartialSurfaceRateLimitPolicy {
                        max_requests: Some(91),
                        window_seconds: None,
                    }),
                    v1_models: None,
                    v1_chat_completions: None,
                }),
                idempotency: Some(PartialIdempotencyConfig {
                    chat_completions: Some(PartialChatCompletionsIdempotencyConfig {
                        enabled: None,
                        replay_window_seconds: Some(3600),
                    }),
                }),
            }),
        })
        .expect("effective config should assemble");

        assert_eq!(config.host, "3.3.3.3");
        assert_eq!(config.port, 8484);
        assert!(config.enable_tui);
        assert_eq!(config.db_path, std::path::PathBuf::from("/cli/rook.db"));
        assert_eq!(
            config.inbound_auth.bearer_token.as_deref(),
            Some("cli-token")
        );
        assert_eq!(config.rate_limits.api.max_requests, 91);
        assert_eq!(config.rate_limits.api.window_seconds, 41);
        assert_eq!(
            config.idempotency.chat_completions.replay_window_seconds,
            3600
        );
    }

    #[test]
    fn parse_env_overlay_maps_supported_rook_variables_and_ignores_unknown_ones() {
        let env = std::collections::HashMap::from([
            ("ROOK_ENABLE_TUI".to_string(), "yes".to_string()),
            (
                "ROOK_TRANSPORT_TRUSTED_PROXY_ENABLED".to_string(),
                "true".to_string(),
            ),
            (
                "ROOK_TRANSPORT_TRUSTED_PROXY_TRUSTED_CIDRS".to_string(),
                "10.0.0.0/8, 192.168.0.0/16".to_string(),
            ),
            (
                "ROOK_CHAT_IDEMPOTENCY_REPLAY_WINDOW_SECONDS".to_string(),
                "1234".to_string(),
            ),
            ("ROOK_UNSUPPORTED".to_string(), "ignored".to_string()),
        ]);

        let overlay = parse_env_overlay(&env).expect("env overlay should parse");

        assert_eq!(overlay.enable_tui, Some(true));
        assert_eq!(
            overlay
                .transport
                .as_ref()
                .and_then(|transport| transport.trusted_proxy.as_ref())
                .and_then(|proxy| proxy.enabled),
            Some(true)
        );
        assert_eq!(
            overlay
                .transport
                .as_ref()
                .and_then(|transport| transport.trusted_proxy.as_ref())
                .and_then(|proxy| proxy.trusted_cidrs.clone()),
            Some(vec!["10.0.0.0/8".to_string(), "192.168.0.0/16".to_string()])
        );
        assert_eq!(
            overlay
                .idempotency
                .as_ref()
                .and_then(|idempotency| idempotency.chat_completions.as_ref())
                .and_then(|chat| chat.replay_window_seconds),
            Some(1234)
        );
    }

    #[test]
    fn rook_config_export_view_never_serializes_secret_like_literals() {
        let output = serde_json::to_string(&super::RookConfigExportView::from_config(
            &super::RookConfig {
                inbound_auth: InboundAuthConfig {
                    enabled: true,
                    bearer_token: Some("super-secret-token".to_string()),
                },
                ..Default::default()
            },
        ))
        .expect("export view should serialize");

        for forbidden in [
            "super-secret-token",
            "sk-secret",
            "Bearer secret-value",
            "session_cookie=abc123",
        ] {
            assert!(
                !output.contains(forbidden),
                "export output leaked forbidden literal: {forbidden}"
            );
        }
        assert!(output.contains("[redacted]"));
    }

    #[test]
    fn rook_config_export_view_redacts_inbound_auth_token() {
        let config = super::RookConfigExportView::from_config(&super::RookConfig {
            inbound_auth: InboundAuthConfig {
                enabled: true,
                bearer_token: Some("super-secret-token".to_string()),
            },
            ..Default::default()
        });

        assert!(config.inbound_auth.enabled);
        assert_eq!(
            config.inbound_auth.bearer_token.as_deref(),
            Some("[redacted]")
        );
    }

    #[test]
    fn rook_config_export_view_omits_token_when_inbound_auth_is_disabled() {
        let config = super::RookConfigExportView::from_config(&super::RookConfig {
            inbound_auth: InboundAuthConfig {
                enabled: false,
                bearer_token: Some("super-secret-token".to_string()),
            },
            ..Default::default()
        });

        assert!(!config.inbound_auth.enabled);
        assert_eq!(config.inbound_auth.bearer_token, None);
    }

    #[test]
    fn rook_config_export_view_marks_missing_token_as_not_configured() {
        let config = super::RookConfigExportView::from_config(&super::RookConfig {
            inbound_auth: InboundAuthConfig {
                enabled: true,
                bearer_token: None,
            },
            ..Default::default()
        });

        assert_eq!(
            config.inbound_auth.bearer_token.as_deref(),
            Some("[not configured]")
        );
    }

    #[test]
    fn rook_config_export_view_includes_transport_rate_limits_and_idempotency() {
        let config = super::RookConfigExportView::from_config(&super::RookConfig {
            transport: TransportConfig {
                request_id: RequestIdConfig {
                    inbound_header_name: "x-correlation-id".to_string(),
                    response_header_name: "x-correlation-id".to_string(),
                    max_length: 256,
                },
                trusted_proxy: TrustedProxyConfig {
                    enabled: true,
                    trusted_cidrs: vec!["10.0.0.0/8".to_string()],
                    allowed_headers: TrustedForwardedHeaders {
                        forwarded: true,
                        x_forwarded_for: true,
                        x_forwarded_host: false,
                        x_forwarded_proto: true,
                        x_forwarded_port: false,
                        x_real_ip: true,
                    },
                },
            },
            rate_limits: RateLimitConfig {
                api: SurfaceRateLimitPolicy {
                    max_requests: 10,
                    window_seconds: 30,
                },
                v1_models: SurfaceRateLimitPolicy {
                    max_requests: 20,
                    window_seconds: 40,
                },
                v1_chat_completions: SurfaceRateLimitPolicy {
                    max_requests: 5,
                    window_seconds: 50,
                },
            },
            idempotency: IdempotencyConfig {
                chat_completions: ChatCompletionsIdempotencyConfig {
                    enabled: true,
                    replay_window_seconds: 7200,
                },
            },
            ..Default::default()
        });

        assert_eq!(
            config.transport.request_id.inbound_header_name,
            "x-correlation-id"
        );
        assert_eq!(config.transport.request_id.max_length, 256);
        assert!(config.transport.trusted_proxy.enabled);
        assert_eq!(
            config.transport.trusted_proxy.trusted_cidrs,
            vec!["10.0.0.0/8".to_string()]
        );
        assert!(config.transport.trusted_proxy.allowed_headers.forwarded);
        assert_eq!(config.rate_limits.api.max_requests, 10);
        assert_eq!(config.rate_limits.v1_models.window_seconds, 40);
        assert_eq!(config.rate_limits.v1_chat_completions.max_requests, 5);
        assert!(config.idempotency.chat_completions.enabled);
        assert_eq!(
            config.idempotency.chat_completions.replay_window_seconds,
            7200
        );
    }

    #[test]
    fn inbound_auth_debug_redacts_bearer_token() {
        let rendered = format!(
            "{:?}",
            InboundAuthConfig {
                enabled: true,
                bearer_token: Some("super-secret-token".to_string()),
            }
        );

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("super-secret-token"));
    }

    #[test]
    fn parse_bool_env_reports_raw_input() {
        let error = super::parse_bool_env("ROOK_ENABLE_TUI", " true\nmaybe ")
            .expect_err("invalid bool env should fail");

        match error {
            crate::domain::RookError::Config(message) => {
                assert_eq!(message, "invalid ROOK_ENABLE_TUI value ' true\nmaybe '");
            }
            other => panic!("expected config error, got {other:?}"),
        }
    }

    #[test]
    fn rook_config_from_toml_overrides_default_values() {
        let config = super::RookConfig::from_toml_str(
            r#"
            host = "0.0.0.0"
            port = 5151
            enable_tui = true
            db_path = "/tmp/rook-test.db"
            "#,
        )
        .expect("toml config should parse");

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 5151);
        assert!(config.enable_tui);
        assert_eq!(
            config.db_path,
            std::path::PathBuf::from("/tmp/rook-test.db")
        );
    }

    #[test]
    fn rook_config_apply_env_overrides_replaces_supported_fields() {
        let mut config = super::RookConfig::default();
        let env = std::collections::HashMap::from([
            ("ROOK_HOST".to_string(), "0.0.0.0".to_string()),
            ("ROOK_PORT".to_string(), "5252".to_string()),
            ("ROOK_ENABLE_TUI".to_string(), "true".to_string()),
            ("ROOK_DB_PATH".to_string(), "/var/lib/rook.db".to_string()),
        ]);

        config
            .apply_env_overrides(&env)
            .expect("env overrides should apply");

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 5252);
        assert!(config.enable_tui);
        assert_eq!(config.db_path, std::path::PathBuf::from("/var/lib/rook.db"));
    }

    #[test]
    fn rook_config_apply_env_overrides_rejects_invalid_port() {
        let mut config = super::RookConfig::default();
        let env =
            std::collections::HashMap::from([("ROOK_PORT".to_string(), "not-a-port".to_string())]);

        let error = config
            .apply_env_overrides(&env)
            .expect_err("invalid port should fail");

        assert!(error.to_string().contains("ROOK_PORT"));
    }

    #[test]
    fn rook_config_to_server_config_preserves_phase_1_fields() {
        let server_config = super::RookConfig {
            host: "0.0.0.0".to_string(),
            port: 6262,
            enable_tui: true,
            db_path: std::path::PathBuf::from("/tmp/rook-config.db"),
            ..Default::default()
        }
        .to_server_config();

        assert_eq!(server_config.host, "0.0.0.0");
        assert_eq!(server_config.port, 6262);
        assert!(server_config.enable_tui);
        assert_eq!(
            server_config.db_path.as_deref(),
            Some("/tmp/rook-config.db")
        );
    }

    #[test]
    fn rook_config_validate_reuses_subconfig_validation() {
        let config = super::RookConfig {
            host: "   ".to_string(),
            inbound_auth: InboundAuthConfig {
                enabled: true,
                bearer_token: None,
            },
            ..Default::default()
        };

        let error = config.validate().expect_err("invalid config should fail");
        match error {
            crate::domain::RookError::Config(message) => {
                assert!(message.contains("server host") || message.contains("bearer token"));
            }
            other => panic!("expected config error, got {other}"),
        }
    }

    #[test]
    fn rook_config_from_sources_applies_defaults_then_file_then_env() {
        let env = std::collections::HashMap::from([
            ("ROOK_PORT".to_string(), "7373".to_string()),
            ("ROOK_DB_PATH".to_string(), "/env/rook.db".to_string()),
        ]);

        let config = super::RookConfig::from_sources(
            Some(
                r#"
                host = "0.0.0.0"
                port = 6363
                db_path = "/file/rook.db"
                "#,
            ),
            &env,
        )
        .expect("config sources should assemble");

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 7373);
        assert_eq!(config.db_path, std::path::PathBuf::from("/env/rook.db"));
    }

    #[test]
    fn rook_config_from_toml_parses_inbound_auth_and_rate_limits() {
        let config = super::RookConfig::from_toml_str(
            r#"
            [inbound_auth]
            enabled = true
            bearer_token = "file-token"

            [rate_limits.api]
            max_requests = 71
            window_seconds = 41

            [rate_limits.v1_models]
            max_requests = 72
            window_seconds = 42
            "#,
        )
        .expect("toml config should parse nested fields");

        assert!(config.inbound_auth.enabled);
        assert_eq!(
            config.inbound_auth.bearer_token.as_deref(),
            Some("file-token")
        );
        assert_eq!(config.rate_limits.api.max_requests, 71);
        assert_eq!(config.rate_limits.api.window_seconds, 41);
        assert_eq!(config.rate_limits.v1_models.max_requests, 72);
        assert_eq!(config.rate_limits.v1_models.window_seconds, 42);
    }

    #[test]
    fn rook_config_apply_env_overrides_updates_inbound_auth_and_rate_limits() {
        let mut config = super::RookConfig::default();
        let env = std::collections::HashMap::from([
            ("ROOK_INBOUND_AUTH_ENABLED".to_string(), "true".to_string()),
            (
                "ROOK_INBOUND_AUTH_TOKEN".to_string(),
                "env-token".to_string(),
            ),
            (
                "ROOK_API_RATE_LIMIT_MAX_REQUESTS".to_string(),
                "81".to_string(),
            ),
            (
                "ROOK_API_RATE_LIMIT_WINDOW_SECONDS".to_string(),
                "51".to_string(),
            ),
        ]);

        config
            .apply_env_overrides(&env)
            .expect("env overrides should apply nested fields");

        assert!(config.inbound_auth.enabled);
        assert_eq!(
            config.inbound_auth.bearer_token.as_deref(),
            Some("env-token")
        );
        assert_eq!(config.rate_limits.api.max_requests, 81);
        assert_eq!(config.rate_limits.api.window_seconds, 51);
    }

    #[test]
    fn rook_config_from_toml_parses_remaining_rate_limits_and_idempotency() {
        let config = super::RookConfig::from_toml_str(
            r#"
            [rate_limits.v1_chat_completions]
            max_requests = 91
            window_seconds = 61

            [idempotency.chat_completions]
            enabled = false
            replay_window_seconds = 1234
            "#,
        )
        .expect("toml config should parse idempotency fields");

        assert_eq!(config.rate_limits.v1_chat_completions.max_requests, 91);
        assert_eq!(config.rate_limits.v1_chat_completions.window_seconds, 61);
        assert!(!config.idempotency.chat_completions.enabled);
        assert_eq!(
            config.idempotency.chat_completions.replay_window_seconds,
            1234
        );
    }

    #[test]
    fn rook_config_apply_env_overrides_updates_remaining_rate_limits_and_idempotency() {
        let mut config = super::RookConfig::default();
        let env = std::collections::HashMap::from([
            (
                "ROOK_V1_MODELS_RATE_LIMIT_MAX_REQUESTS".to_string(),
                "82".to_string(),
            ),
            (
                "ROOK_V1_MODELS_RATE_LIMIT_WINDOW_SECONDS".to_string(),
                "52".to_string(),
            ),
            (
                "ROOK_V1_CHAT_RATE_LIMIT_MAX_REQUESTS".to_string(),
                "83".to_string(),
            ),
            (
                "ROOK_V1_CHAT_RATE_LIMIT_WINDOW_SECONDS".to_string(),
                "53".to_string(),
            ),
            (
                "ROOK_CHAT_IDEMPOTENCY_ENABLED".to_string(),
                "false".to_string(),
            ),
            (
                "ROOK_CHAT_IDEMPOTENCY_REPLAY_WINDOW_SECONDS".to_string(),
                "4321".to_string(),
            ),
        ]);

        config
            .apply_env_overrides(&env)
            .expect("env overrides should apply idempotency fields");

        assert_eq!(config.rate_limits.v1_models.max_requests, 82);
        assert_eq!(config.rate_limits.v1_models.window_seconds, 52);
        assert_eq!(config.rate_limits.v1_chat_completions.max_requests, 83);
        assert_eq!(config.rate_limits.v1_chat_completions.window_seconds, 53);
        assert!(!config.idempotency.chat_completions.enabled);
        assert_eq!(
            config.idempotency.chat_completions.replay_window_seconds,
            4321
        );
    }

    #[test]
    fn rook_config_loads_from_file_path() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let config_path = temp_dir.path().join("rook.toml");
        std::fs::write(
            &config_path,
            r#"
            host = "0.0.0.0"
            [inbound_auth]
            enabled = true
            bearer_token = "file-token"
            "#,
        )
        .expect("config file should be written");

        let config = super::RookConfig::load_from_file(&config_path)
            .expect("config should load from file path");

        assert_eq!(config.host, "0.0.0.0");
        assert!(config.inbound_auth.enabled);
        assert_eq!(
            config.inbound_auth.bearer_token.as_deref(),
            Some("file-token")
        );
    }

    #[test]
    fn rook_config_load_from_file_returns_default_when_path_missing() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let missing_path = temp_dir.path().join("missing.toml");

        let config = super::RookConfig::load_from_optional_file(Some(&missing_path))
            .expect("missing config path should fall back to default");

        assert_eq!(config, super::RookConfig::default());
    }

    #[test]
    fn discover_default_config_path_uses_xdg_config_home() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let env = std::collections::HashMap::from([(
            "XDG_CONFIG_HOME".to_string(),
            temp_dir.path().display().to_string(),
        )]);

        let path =
            super::discover_default_config_path(&env).expect("default config path should resolve");

        assert_eq!(path, temp_dir.path().join("rook").join("config.toml"));
    }

    #[test]
    fn rook_config_from_toml_parses_transport_fields() {
        let config = super::RookConfig::from_toml_str(
            r#"
            [transport.request_id]
            inbound_header_name = "x-correlation-id"
            response_header_name = "x-correlation-id"
            max_length = 64

            [transport.trusted_proxy]
            enabled = true
            trusted_cidrs = ["10.0.0.0/8"]

            [transport.trusted_proxy.allowed_headers]
            forwarded = true
            x_forwarded_for = true
            x_real_ip = true
            "#,
        )
        .expect("toml transport config should parse");

        assert_eq!(
            config.transport.request_id.inbound_header_name,
            "x-correlation-id"
        );
        assert_eq!(
            config.transport.request_id.response_header_name,
            "x-correlation-id"
        );
        assert_eq!(config.transport.request_id.max_length, 64);
        assert!(config.transport.trusted_proxy.enabled);
        assert_eq!(
            config.transport.trusted_proxy.trusted_cidrs,
            vec!["10.0.0.0/8"]
        );
        assert!(config.transport.trusted_proxy.allowed_headers.forwarded);
        assert!(
            config
                .transport
                .trusted_proxy
                .allowed_headers
                .x_forwarded_for
        );
        assert!(config.transport.trusted_proxy.allowed_headers.x_real_ip);
    }

    #[test]
    fn rook_config_apply_env_overrides_updates_transport_fields() {
        let mut config = super::RookConfig::default();
        let env = std::collections::HashMap::from([
            (
                "ROOK_TRANSPORT_REQUEST_ID_INBOUND_HEADER_NAME".to_string(),
                "x-correlation-id".to_string(),
            ),
            (
                "ROOK_TRANSPORT_REQUEST_ID_RESPONSE_HEADER_NAME".to_string(),
                "x-correlation-id".to_string(),
            ),
            (
                "ROOK_TRANSPORT_REQUEST_ID_MAX_LENGTH".to_string(),
                "64".to_string(),
            ),
            (
                "ROOK_TRANSPORT_TRUSTED_PROXY_ENABLED".to_string(),
                "true".to_string(),
            ),
            (
                "ROOK_TRANSPORT_TRUSTED_PROXY_TRUSTED_CIDRS".to_string(),
                "10.0.0.0/8,192.168.0.0/16".to_string(),
            ),
            (
                "ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_FORWARDED".to_string(),
                "true".to_string(),
            ),
        ]);

        config
            .apply_env_overrides(&env)
            .expect("transport env overrides should apply");

        assert_eq!(
            config.transport.request_id.inbound_header_name,
            "x-correlation-id"
        );
        assert_eq!(
            config.transport.request_id.response_header_name,
            "x-correlation-id"
        );
        assert_eq!(config.transport.request_id.max_length, 64);
        assert!(config.transport.trusted_proxy.enabled);
        assert_eq!(
            config.transport.trusted_proxy.trusted_cidrs,
            vec!["10.0.0.0/8".to_string(), "192.168.0.0/16".to_string()]
        );
        assert!(config.transport.trusted_proxy.allowed_headers.forwarded);
    }

    fn policy(max_requests: u32, window_seconds: u64) -> SurfaceRateLimitPolicy {
        SurfaceRateLimitPolicy {
            max_requests,
            window_seconds,
        }
    }

    #[test]
    fn inbound_auth_config_validate_rejects_enabled_missing_token() {
        let config = InboundAuthConfig {
            enabled: true,
            bearer_token: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn inbound_auth_config_validate_rejects_enabled_blank_token() {
        let config = InboundAuthConfig {
            enabled: true,
            bearer_token: Some("   ".to_string()),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn inbound_auth_config_validate_accepts_enabled_valid_token() {
        let config = InboundAuthConfig {
            enabled: true,
            bearer_token: Some("rook-inbound-secret".to_string()),
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn inbound_auth_operator_state_reports_enabled_and_configured_without_exposing_token() {
        let config = InboundAuthConfig {
            enabled: true,
            bearer_token: Some("rook-inbound-secret".to_string()),
        };

        let state = config.operator_state();

        assert_eq!(
            state,
            InboundAuthOperatorState {
                enabled: true,
                token_configured: true,
            }
        );
        let rendered = format!("{state:?}");
        assert!(!rendered.contains("rook-inbound-secret"));
    }

    #[test]
    fn inbound_auth_operator_state_treats_blank_token_as_not_configured() {
        let config = InboundAuthConfig {
            enabled: true,
            bearer_token: Some("   ".to_string()),
        };

        assert_eq!(
            config.operator_state(),
            InboundAuthOperatorState {
                enabled: true,
                token_configured: false,
            }
        );
    }

    #[test]
    fn inbound_auth_config_validate_allows_disabled_missing_token() {
        let config = InboundAuthConfig {
            enabled: false,
            bearer_token: None,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn idempotency_config_defaults_to_enabled_chat_replay_window() {
        let config = IdempotencyConfig::default();

        assert_eq!(
            config.chat_completions,
            ChatCompletionsIdempotencyConfig {
                enabled: true,
                replay_window_seconds: 86_400,
            }
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn idempotency_config_rejects_zero_replay_window() {
        let config = IdempotencyConfig {
            chat_completions: ChatCompletionsIdempotencyConfig {
                enabled: true,
                replay_window_seconds: 0,
            },
        };

        let error = config
            .validate()
            .expect_err("zero replay window must fail validation");
        assert!(error
            .to_string()
            .contains("replay window must be greater than zero"));
    }

    #[test]
    fn transport_config_defaults_to_strict_request_id_and_disabled_proxy_trust() {
        let config = TransportConfig::default();

        assert_eq!(
            config.request_id,
            RequestIdConfig {
                inbound_header_name: "x-request-id".to_string(),
                response_header_name: "x-request-id".to_string(),
                max_length: 128,
            }
        );
        assert_eq!(
            config.trusted_proxy,
            TrustedProxyConfig {
                enabled: false,
                trusted_cidrs: vec![],
                allowed_headers: TrustedForwardedHeaders::default(),
            }
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn transport_config_validate_rejects_enabled_proxy_without_cidrs() {
        let config = TransportConfig {
            trusted_proxy: TrustedProxyConfig {
                enabled: true,
                trusted_cidrs: vec![],
                allowed_headers: TrustedForwardedHeaders {
                    x_forwarded_for: true,
                    ..TrustedForwardedHeaders::default()
                },
            },
            ..TransportConfig::default()
        };

        let error = config
            .validate()
            .expect_err("proxy trust without cidrs must fail");
        assert!(error
            .to_string()
            .contains("trusted proxy CIDR list must not be empty"));
    }

    #[test]
    fn transport_config_validate_rejects_invalid_trusted_proxy_cidr() {
        let config = TransportConfig {
            trusted_proxy: TrustedProxyConfig {
                enabled: true,
                trusted_cidrs: vec!["not-a-cidr".to_string()],
                allowed_headers: TrustedForwardedHeaders {
                    x_forwarded_for: true,
                    ..TrustedForwardedHeaders::default()
                },
            },
            ..TransportConfig::default()
        };

        let error = config
            .validate()
            .expect_err("invalid cidr must fail closed");
        assert!(error.to_string().contains("invalid trusted proxy CIDR"));
    }

    #[test]
    fn rate_limit_config_validate_accepts_explicit_valid_surface_policies() {
        let config = RateLimitConfig {
            api: policy(60, 60),
            v1_models: policy(120, 60),
            v1_chat_completions: policy(30, 60),
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn rate_limit_config_deserialization_fails_closed_when_any_surface_is_missing() {
        let error = serde_json::from_value::<RateLimitConfig>(json!({
            "api": { "max_requests": 60, "window_seconds": 60 },
            "v1_models": { "max_requests": 120, "window_seconds": 60 }
        }))
        .expect_err("missing chat surface must fail closed");

        assert!(error.to_string().contains("v1_chat_completions"));
    }

    #[test]
    fn rate_limit_config_validate_rejects_zero_or_malformed_surface_values() {
        let zero_requests = RateLimitConfig {
            api: policy(0, 60),
            v1_models: policy(120, 60),
            v1_chat_completions: policy(30, 60),
        };
        assert!(zero_requests.validate().is_err());

        let zero_window = RateLimitConfig {
            api: policy(60, 0),
            v1_models: policy(120, 60),
            v1_chat_completions: policy(30, 60),
        };
        assert!(zero_window.validate().is_err());

        let malformed = serde_json::from_value::<RateLimitConfig>(json!({
            "api": { "max_requests": "a-lot", "window_seconds": 60 },
            "v1_models": { "max_requests": 120, "window_seconds": 60 },
            "v1_chat_completions": { "max_requests": 30, "window_seconds": 60 }
        }))
        .expect_err("string max_requests must be rejected");
        let message = malformed.to_string();
        assert!(message.contains("max_requests") || message.contains("invalid type"));
    }
}
