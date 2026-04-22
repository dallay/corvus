use axum::http::HeaderMap;

use crate::auth::bearer::{extract_bearer_token, BearerExtractionError};
use crate::config::InboundAuthConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    pub scope_id: String,
}

impl AuthenticatedPrincipal {
    pub fn from_inbound_auth(
        headers: &HeaderMap,
        config: &InboundAuthConfig,
    ) -> Result<Self, InboundAuthFailure> {
        if !config.enabled {
            return Ok(Self {
                scope_id: "anonymous-local".to_string(),
            });
        }

        validate_inbound_request(headers, config)?;
        let token = extract_bearer_token(headers).map_err(|error| match error {
            BearerExtractionError::Missing => InboundAuthFailure::Missing,
            BearerExtractionError::InvalidScheme
            | BearerExtractionError::EmptyToken
            | BearerExtractionError::Malformed => InboundAuthFailure::Malformed,
        })?;

        Ok(Self {
            scope_id: token.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboundAuthFailure {
    Missing,
    Malformed,
    Invalid,
}

pub fn validate_inbound_request(
    headers: &HeaderMap,
    config: &InboundAuthConfig,
) -> Result<(), InboundAuthFailure> {
    if !config.enabled {
        return Ok(());
    }

    let expected = config
        .bearer_token
        .as_deref()
        .filter(|token| !token.trim().is_empty())
        .ok_or(InboundAuthFailure::Invalid)?;

    match extract_bearer_token(headers) {
        Ok(token) if token == expected => Ok(()),
        Ok(_) => Err(InboundAuthFailure::Invalid),
        Err(BearerExtractionError::Missing) => Err(InboundAuthFailure::Missing),
        Err(BearerExtractionError::InvalidScheme)
        | Err(BearerExtractionError::EmptyToken)
        | Err(BearerExtractionError::Malformed) => Err(InboundAuthFailure::Malformed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn authenticated_principal_uses_bearer_token_scope_when_auth_enabled() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer rook-secret"),
        );
        let config = InboundAuthConfig {
            enabled: true,
            bearer_token: Some("rook-secret".to_string()),
        };

        let principal = AuthenticatedPrincipal::from_inbound_auth(&headers, &config)
            .expect("valid bearer token should produce a principal");

        assert_eq!(
            principal,
            AuthenticatedPrincipal {
                scope_id: "rook-secret".to_string(),
            }
        );
    }

    #[test]
    fn authenticated_principal_uses_local_scope_when_auth_disabled() {
        let headers = HeaderMap::new();
        let config = InboundAuthConfig {
            enabled: false,
            bearer_token: None,
        };

        let principal = AuthenticatedPrincipal::from_inbound_auth(&headers, &config)
            .expect("disabled auth should use the local principal scope");

        assert_eq!(
            principal,
            AuthenticatedPrincipal {
                scope_id: "anonymous-local".to_string(),
            }
        );
    }
}
