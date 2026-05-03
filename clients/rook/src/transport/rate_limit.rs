use crate::admin::types::admin_rate_limited_response;
use crate::auth::types::AuthenticatedPrincipal;
use crate::config::{RateLimitConfig, SurfaceRateLimitPolicy};
use crate::gateway::types::gateway_rate_limited_response;
use crate::observability::{
    normalize_rate_limit_endpoint, normalize_rate_limit_surface, Observability,
};
use crate::transport::context::{ForwardedTrust, RateLimitedSurface, SanitizedTransportContext};
use axum::body::Body;
use axum::extract::{MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::Response;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Probability-based pruning: 1% chance to trigger pruning on each check.
const PRUNE_CHANCE_DENOMINATOR: u32 = 100;
const PRUNE_CHANCE_NUMERATOR: u32 = 1;

/// TTL for stale entry eviction: 2x the max configured window (safe upper bound).
/// Window configs max out at 3600s (1 hour), so 7200s is safe.
const MAX_TTL_SECONDS: u64 = 7200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitDecision {
    Allow,
    Reject { retry_after_seconds: u64 },
}

#[derive(Debug, Clone)]
pub struct SurfaceWindowState {
    pub window_started_at: Instant,
    pub request_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RateLimitPrincipalKey {
    Authenticated(String),
    TrustedForwardedIp(std::net::IpAddr),
    DirectIp(std::net::IpAddr),
    LocalAnonymous,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RateLimitBucketKey {
    pub surface: RateLimitedSurface,
    pub principal: RateLimitPrincipalKey,
}

#[derive(Debug, Clone)]
pub struct RateLimitState {
    pub policies: HashMap<RateLimitedSurface, SurfaceRateLimitPolicy>,
    pub windows: Arc<tokio::sync::Mutex<HashMap<RateLimitBucketKey, SurfaceWindowState>>>,
}

#[derive(Debug, Clone)]
pub struct RateLimitMiddlewareState {
    pub state: RateLimitState,
    pub surface: RateLimitedSurface,
    pub observability: Arc<Observability>,
}

impl RateLimitState {
    pub fn new(config: &RateLimitConfig) -> Self {
        let policies = HashMap::from([
            (RateLimitedSurface::AdminApi, config.api.clone()),
            (RateLimitedSurface::GatewayModels, config.v1_models.clone()),
            (
                RateLimitedSurface::GatewayChatCompletions,
                config.v1_chat_completions.clone(),
            ),
        ]);
        Self {
            policies,
            windows: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    pub async fn check(
        &self,
        surface: RateLimitedSurface,
        principal: RateLimitPrincipalKey,
    ) -> RateLimitDecision {
        let now = Instant::now();
        let policy = self
            .policies
            .get(&surface)
            .expect("covered surface policy must exist");
        let key = RateLimitBucketKey { surface, principal };

        // Probabilistic pruning to evict stale entries and prevent unbounded growth.
        let mut windows = self.windows.lock().await;
        if should_prune() {
            pruning(&mut windows, now);
        }

        let window = windows.entry(key).or_insert_with(|| SurfaceWindowState {
            window_started_at: now,
            request_count: 0,
        });
        evaluate_surface_limit(now, policy, window)
    }
}

/// Returns true with ~1% probability per call.
/// Uses thread-local RNG to avoid per-call overhead.
fn should_prune() -> bool {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let state = RandomState::new();
    let hash = state.build_hasher();
    let seed = hash.finish();
    (seed % u64::from(PRUNE_CHANCE_DENOMINATOR)) < u64::from(PRUNE_CHANCE_NUMERATOR)
}

/// Removes entries whose window started more than `ttl` seconds ago.
/// Uses `ttl = max_window_seconds * 2` as a safe upper bound.
fn pruning(windows: &mut HashMap<RateLimitBucketKey, SurfaceWindowState>, now: Instant) {
    let ttl = Duration::from_secs(MAX_TTL_SECONDS);
    windows.retain(|_, state| now.saturating_duration_since(state.window_started_at) < ttl);
}

pub fn resolve_rate_limit_principal(request: &Request<Body>) -> RateLimitPrincipalKey {
    if let Some(principal) = request.extensions().get::<AuthenticatedPrincipal>() {
        if principal.scope_id == "anonymous-local" {
            return RateLimitPrincipalKey::LocalAnonymous;
        }
        return RateLimitPrincipalKey::Authenticated(principal.scope_id.clone());
    }

    let Some(context) = request.extensions().get::<SanitizedTransportContext>() else {
        return RateLimitPrincipalKey::Unknown;
    };

    if context.forwarded.trust == ForwardedTrust::Trusted {
        if let Some(client_ip) = context.forwarded.client_ip {
            return RateLimitPrincipalKey::TrustedForwardedIp(client_ip);
        }
    }

    if let Some(peer_addr) = context.direct_peer_addr {
        return RateLimitPrincipalKey::DirectIp(peer_addr.ip());
    }

    RateLimitPrincipalKey::Unknown
}

fn normalized_endpoint(request: &Request<Body>, surface: RateLimitedSurface) -> String {
    normalize_rate_limit_endpoint(
        surface,
        request
            .extensions()
            .get::<MatchedPath>()
            .map(MatchedPath::as_str),
    )
    .into_owned()
}

pub fn evaluate_surface_limit(
    now: Instant,
    policy: &SurfaceRateLimitPolicy,
    window: &mut SurfaceWindowState,
) -> RateLimitDecision {
    let window_duration = Duration::from_secs(policy.window_seconds);

    if now.saturating_duration_since(window.window_started_at) >= window_duration {
        window.window_started_at = now;
        window.request_count = 0;
    }

    if window.request_count < policy.max_requests {
        window.request_count += 1;
        return RateLimitDecision::Allow;
    }

    let window_ends_at = window.window_started_at + window_duration;
    let remaining = window_ends_at.saturating_duration_since(now);
    let retry_after_seconds = remaining.as_secs().max(1);

    RateLimitDecision::Reject {
        retry_after_seconds,
    }
}

pub async fn apply_rate_limit(
    State(state): State<RateLimitMiddlewareState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let endpoint = normalized_endpoint(&request, state.surface);
    if matches!(endpoint.as_str(), "/metrics" | "/api/metrics") {
        return next.run(request).await;
    }

    let surface = normalize_rate_limit_surface(state.surface);
    let principal = resolve_rate_limit_principal(&request);
    match state.state.check(state.surface, principal).await {
        RateLimitDecision::Allow => {
            state
                .observability
                .rate_limit_outcomes_total()
                .inc(surface, endpoint.clone(), "allow");
            next.run(request).await
        }
        RateLimitDecision::Reject {
            retry_after_seconds,
        } => {
            state.observability.rate_limit_outcomes_total().inc(
                surface,
                endpoint.clone(),
                "reject",
            );
            match state.surface {
                RateLimitedSurface::AdminApi => admin_rate_limited_response(retry_after_seconds),
                RateLimitedSurface::GatewayModels | RateLimitedSurface::GatewayChatCompletions => {
                    gateway_rate_limited_response(retry_after_seconds)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_surface_limit, pruning, resolve_rate_limit_principal, RateLimitBucketKey,
        RateLimitDecision, RateLimitPrincipalKey, RateLimitState, SurfaceWindowState,
    };
    use crate::config::{RateLimitConfig, SurfaceRateLimitPolicy};
    use crate::transport::context::{
        ForwardedTrust, RateLimitedSurface, RouteSurface, SanitizedForwardedContext,
        SanitizedTransportContext,
    };
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    fn policy(max_requests: u32, window_seconds: u64) -> SurfaceRateLimitPolicy {
        SurfaceRateLimitPolicy {
            max_requests,
            window_seconds,
        }
    }

    fn request_with_context(
        forwarded_trust: ForwardedTrust,
        forwarded_ip: Option<&str>,
        direct_peer: Option<&str>,
    ) -> axum::http::Request<axum::body::Body> {
        let mut request = axum::http::Request::builder()
            .body(axum::body::Body::empty())
            .unwrap();
        request.extensions_mut().insert(SanitizedTransportContext {
            request_id: "req-1".to_string(),
            route_surface: RouteSurface::GatewayV1,
            direct_peer_addr: direct_peer.map(|peer| peer.parse().unwrap()),
            forwarded: SanitizedForwardedContext {
                trust: forwarded_trust,
                client_ip: forwarded_ip.map(|ip| ip.parse().unwrap()),
                host: None,
                proto: None,
                port: None,
                ignored_headers: vec![],
            },
        });
        request
    }

    #[test]
    fn evaluate_surface_limit_allows_within_budget_then_rejects_with_retry_after() {
        let now = Instant::now();
        let policy = policy(1, 60);
        let mut window = SurfaceWindowState {
            window_started_at: now,
            request_count: 0,
        };

        assert_eq!(
            evaluate_surface_limit(now, &policy, &mut window),
            RateLimitDecision::Allow
        );
        assert_eq!(window.request_count, 1);

        assert_eq!(
            evaluate_surface_limit(now + Duration::from_secs(1), &policy, &mut window),
            RateLimitDecision::Reject {
                retry_after_seconds: 59,
            }
        );
    }

    #[test]
    fn evaluate_surface_limit_resets_window_after_expiration() {
        let started_at = Instant::now();
        let policy = policy(1, 60);
        let mut window = SurfaceWindowState {
            window_started_at: started_at,
            request_count: 1,
        };

        assert_eq!(
            evaluate_surface_limit(started_at + Duration::from_secs(60), &policy, &mut window),
            RateLimitDecision::Allow
        );
        assert_eq!(window.request_count, 1);
        assert_eq!(
            window.window_started_at,
            started_at + Duration::from_secs(60)
        );
    }

    #[test]
    fn pruning_keeps_future_windows_without_panicking() {
        let now = Instant::now();
        let mut windows = HashMap::from([(
            RateLimitBucketKey {
                surface: RateLimitedSurface::AdminApi,
                principal: RateLimitPrincipalKey::Unknown,
            },
            SurfaceWindowState {
                window_started_at: now + Duration::from_secs(1),
                request_count: 1,
            },
        )]);

        pruning(&mut windows, now);

        assert_eq!(windows.len(), 1);
    }

    #[test]
    fn evaluate_surface_limit_handles_future_window_without_panicking() {
        let now = Instant::now();
        let policy = policy(2, 60);
        let mut window = SurfaceWindowState {
            window_started_at: now + Duration::from_secs(1),
            request_count: 0,
        };

        assert_eq!(
            evaluate_surface_limit(now, &policy, &mut window),
            RateLimitDecision::Allow
        );
        assert_eq!(window.request_count, 1);
    }

    #[test]
    fn evaluate_surface_limit_clamps_retry_after_to_at_least_one_second() {
        let started_at = Instant::now();
        let policy = policy(1, 60);
        let mut window = SurfaceWindowState {
            window_started_at: started_at,
            request_count: 1,
        };

        assert_eq!(
            evaluate_surface_limit(
                started_at + Duration::from_secs(59) + Duration::from_millis(900),
                &policy,
                &mut window,
            ),
            RateLimitDecision::Reject {
                retry_after_seconds: 1,
            }
        );
    }

    #[test]
    fn resolve_rate_limit_principal_prefers_authenticated_principal() {
        let mut request = request_with_context(
            ForwardedTrust::Trusted,
            Some("203.0.113.9"),
            Some("198.51.100.10:1234"),
        );
        request
            .extensions_mut()
            .insert(crate::auth::types::AuthenticatedPrincipal {
                scope_id: "token-a".to_string(),
            });

        assert_eq!(
            resolve_rate_limit_principal(&request),
            RateLimitPrincipalKey::Authenticated("token-a".to_string())
        );
    }

    #[test]
    fn resolve_rate_limit_principal_uses_trusted_forwarded_ip_before_direct_peer() {
        let request = request_with_context(
            ForwardedTrust::Trusted,
            Some("203.0.113.9"),
            Some("198.51.100.10:1234"),
        );

        assert_eq!(
            resolve_rate_limit_principal(&request),
            RateLimitPrincipalKey::TrustedForwardedIp("203.0.113.9".parse().unwrap())
        );
    }

    #[test]
    fn resolve_rate_limit_principal_ignores_untrusted_forwarded_ip_and_uses_direct_peer() {
        let request = request_with_context(
            ForwardedTrust::Ignored,
            Some("203.0.113.9"),
            Some("198.51.100.10:1234"),
        );

        assert_eq!(
            resolve_rate_limit_principal(&request),
            RateLimitPrincipalKey::DirectIp("198.51.100.10".parse().unwrap())
        );
    }

    #[test]
    fn resolve_rate_limit_principal_uses_unknown_when_no_context_exists() {
        let request = axum::http::Request::builder()
            .body(axum::body::Body::empty())
            .unwrap();

        assert_eq!(
            resolve_rate_limit_principal(&request),
            RateLimitPrincipalKey::Unknown
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rate_limit_state_keeps_surface_budgets_independent() {
        let state = RateLimitState::new(&RateLimitConfig {
            api: policy(1, 60),
            v1_models: policy(1, 60),
            v1_chat_completions: policy(1, 60),
        });

        assert_eq!(
            state
                .check(RateLimitedSurface::AdminApi, RateLimitPrincipalKey::Unknown)
                .await,
            RateLimitDecision::Allow
        );
        assert_eq!(
            state
                .check(RateLimitedSurface::AdminApi, RateLimitPrincipalKey::Unknown)
                .await,
            RateLimitDecision::Reject {
                retry_after_seconds: 59,
            }
        );

        assert_eq!(
            state
                .check(
                    RateLimitedSurface::GatewayModels,
                    RateLimitPrincipalKey::Unknown
                )
                .await,
            RateLimitDecision::Allow
        );
        assert_eq!(
            state
                .check(
                    RateLimitedSurface::GatewayChatCompletions,
                    RateLimitPrincipalKey::Unknown,
                )
                .await,
            RateLimitDecision::Allow
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rate_limit_state_keeps_principal_budgets_independent_on_same_surface() {
        let state = RateLimitState::new(&RateLimitConfig {
            api: policy(1, 60),
            v1_models: policy(1, 60),
            v1_chat_completions: policy(1, 60),
        });

        let alice = RateLimitPrincipalKey::Authenticated("alice".to_string());
        let bob = RateLimitPrincipalKey::Authenticated("bob".to_string());

        assert_eq!(
            state
                .check(RateLimitedSurface::GatewayChatCompletions, alice.clone())
                .await,
            RateLimitDecision::Allow
        );
        assert!(matches!(
            state
                .check(RateLimitedSurface::GatewayChatCompletions, alice)
                .await,
            RateLimitDecision::Reject { .. }
        ));
        assert_eq!(
            state
                .check(RateLimitedSurface::GatewayChatCompletions, bob)
                .await,
            RateLimitDecision::Allow
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rate_limit_state_keeps_surface_budgets_independent_for_same_principal() {
        let state = RateLimitState::new(&RateLimitConfig {
            api: policy(1, 60),
            v1_models: policy(1, 60),
            v1_chat_completions: policy(1, 60),
        });
        let principal = RateLimitPrincipalKey::Authenticated("shared".to_string());

        assert_eq!(
            state
                .check(RateLimitedSurface::AdminApi, principal.clone())
                .await,
            RateLimitDecision::Allow
        );
        assert!(matches!(
            state
                .check(RateLimitedSurface::AdminApi, principal.clone())
                .await,
            RateLimitDecision::Reject { .. }
        ));
        assert_eq!(
            state
                .check(RateLimitedSurface::GatewayModels, principal)
                .await,
            RateLimitDecision::Allow
        );
    }
}
