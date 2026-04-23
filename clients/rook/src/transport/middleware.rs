use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::middleware::Next;
use axum::response::Response;
use tracing::info;

use crate::config::TransportConfig;
use crate::transport::context::{ForwardedTrust, RouteSurface, SanitizedTransportContext};
use crate::transport::forwarded::resolve_forwarded_context;
use crate::transport::request_id::{resolve_request_id, set_response_request_id_header};

#[derive(Debug, Clone)]
pub struct TransportMiddlewareState {
    pub config: Arc<TransportConfig>,
    pub surface: RouteSurface,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompletionLogFields {
    request_id: String,
    surface: RouteSurface,
    method: String,
    route: String,
    status: u16,
    duration_ms: u64,
    forwarded_trust: ForwardedTrust,
    forwarded_present: bool,
    ignored_forwarded_headers: Vec<&'static str>,
}

struct CompletionLogInput<'a> {
    request_id: &'a str,
    surface: RouteSurface,
    method: &'a axum::http::Method,
    route: &'a str,
    status: axum::http::StatusCode,
    duration_ms: u64,
    forwarded_trust: ForwardedTrust,
    forwarded_present: bool,
    ignored_forwarded_headers: &'a [&'static str],
}

fn build_completion_log_fields(input: CompletionLogInput<'_>) -> CompletionLogFields {
    CompletionLogFields {
        request_id: input.request_id.to_string(),
        surface: input.surface,
        method: input.method.to_string(),
        route: input.route.to_string(),
        status: input.status.as_u16(),
        duration_ms: input.duration_ms,
        forwarded_trust: input.forwarded_trust,
        forwarded_present: input.forwarded_present,
        ignored_forwarded_headers: input.ignored_forwarded_headers.to_vec(),
    }
}

pub async fn apply_transport_baseline(
    State(state): State<TransportMiddlewareState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let started_at = Instant::now();
    let request_id = resolve_request_id(request.headers(), &state.config.request_id);
    let direct_peer_addr = request
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|info| info.0);
    let forwarded = resolve_forwarded_context(
        request.headers(),
        direct_peer_addr,
        &state.config.trusted_proxy,
    );

    request.extensions_mut().insert(SanitizedTransportContext {
        request_id: request_id.effective().to_string(),
        route_surface: state.surface,
        direct_peer_addr,
        forwarded: forwarded.context.clone(),
    });

    let method = request.method().clone();
    let route = request.uri().path().to_string();

    let mut response = next.run(request).await;
    let duration_ms = started_at.elapsed().as_millis() as u64;
    let status = response.status();

    set_response_request_id_header(
        response.headers_mut(),
        request_id.effective(),
        &state.config.request_id,
    );

    let completion = build_completion_log_fields(CompletionLogInput {
        request_id: request_id.effective(),
        surface: state.surface,
        method: &method,
        route: &route,
        status,
        duration_ms,
        forwarded_trust: forwarded.context.trust,
        forwarded_present: forwarded.forwarded_present,
        ignored_forwarded_headers: &forwarded.context.ignored_headers,
    });

    info!(
        request_id = %completion.request_id,
        surface = ?completion.surface,
        method = %completion.method,
        route = %completion.route,
        status = completion.status,
        duration_ms = completion.duration_ms,
        forwarded_trust = ?completion.forwarded_trust,
        forwarded_present = completion.forwarded_present,
        ignored_forwarded_headers = ?completion.ignored_forwarded_headers,
        "completed rook transport request"
    );

    response
}

#[cfg(test)]
mod tests {
    use super::{apply_transport_baseline, TransportMiddlewareState};
    use crate::config::TransportConfig;
    use crate::transport::context::{RouteSurface, SanitizedTransportContext};
    use axum::http::{Request, StatusCode};
    use axum::middleware;
    use axum::response::IntoResponse;
    use axum::routing::get;
    use axum::{Json, Router};
    use serde_json::json;
    use std::sync::Arc;
    use tower::util::ServiceExt;
    fn middleware_state() -> TransportMiddlewareState {
        TransportMiddlewareState {
            config: Arc::new(TransportConfig::default()),
            surface: RouteSurface::GatewayV1,
        }
    }

    fn probe_app() -> Router {
        async fn probe_handler(
            axum::extract::Extension(context): axum::extract::Extension<SanitizedTransportContext>,
        ) -> impl IntoResponse {
            Json(json!({
                "request_id": context.request_id,
                "surface": format!("{:?}", context.route_surface),
                "forwarded_trust": format!("{:?}", context.forwarded.trust),
                "forwarded_host": context.forwarded.host,
            }))
        }

        Router::new()
            .route("/probe", get(probe_handler))
            .layer(middleware::from_fn_with_state(
                middleware_state(),
                apply_transport_baseline,
            ))
    }

    #[tokio::test(flavor = "current_thread")]
    async fn middleware_injects_sanitized_transport_context_before_handler_logic() {
        let response = probe_app()
            .oneshot(
                Request::builder()
                    .uri("/probe")
                    .header("x-request-id", "trace-123")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["request_id"], json!("trace-123"));
        assert_eq!(json["surface"], json!("GatewayV1"));
        assert_eq!(json["forwarded_trust"], json!("Absent"));
    }

    #[test]
    fn middleware_completion_log_fields_remain_structured_and_secret_free() {
        let completion = super::build_completion_log_fields(super::CompletionLogInput {
            request_id: "trace-789",
            surface: RouteSurface::GatewayV1,
            method: &axum::http::Method::GET,
            route: "/probe",
            status: StatusCode::OK,
            duration_ms: 12,
            forwarded_trust: crate::transport::context::ForwardedTrust::Absent,
            forwarded_present: false,
            ignored_forwarded_headers: &[],
        });

        assert_eq!(completion.request_id, "trace-789");
        assert_eq!(completion.method, "GET");
        assert_eq!(completion.route, "/probe");
        assert_eq!(completion.status, 200);
        assert_eq!(completion.duration_ms, 12);
        assert_eq!(
            completion.forwarded_trust,
            crate::transport::context::ForwardedTrust::Absent
        );
        assert!(!completion.forwarded_present);
        let serialized = format!("{completion:?}");
        assert!(!serialized.contains("super-secret"));
    }
}
