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
use axum::http::HeaderName;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InboundAuthConfig {
    pub enabled: bool,
    pub bearer_token: Option<String>,
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
        ChatCompletionsIdempotencyConfig, IdempotencyConfig, InboundAuthConfig,
        InboundAuthOperatorState, RateLimitConfig, RequestIdConfig, SurfaceRateLimitPolicy,
        TransportConfig, TrustedForwardedHeaders, TrustedProxyConfig,
    };
    use serde_json::json;

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
