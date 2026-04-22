use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::Next,
    response::Response,
};

use crate::{
    admin::types::admin_unauthorized_response,
    auth::types::{AuthenticatedPrincipal, validate_inbound_request},
    config::InboundAuthConfig,
    gateway::types::gateway_unauthorized_response,
};

pub async fn admin_inbound_auth(
    State(config): State<InboundAuthConfig>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    match validate_inbound_request(request.headers(), &config) {
        Ok(()) => {
            if let Ok(principal) = AuthenticatedPrincipal::from_inbound_auth(request.headers(), &config)
            {
                request.extensions_mut().insert(principal);
            }
            next.run(request).await
        }
        Err(_) => admin_unauthorized_response(),
    }
}

pub async fn gateway_inbound_auth(
    State(config): State<InboundAuthConfig>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    match validate_inbound_request(request.headers(), &config) {
        Ok(()) => {
            if let Ok(principal) = AuthenticatedPrincipal::from_inbound_auth(request.headers(), &config)
            {
                request.extensions_mut().insert(principal);
            }
            next.run(request).await
        }
        Err(_) => gateway_unauthorized_response(),
    }
}
