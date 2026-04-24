use crate::admin::types::admin_rate_limited_response;
use crate::config::{RateLimitConfig, SurfaceRateLimitPolicy};
use crate::gateway::types::gateway_rate_limited_response;
use crate::transport::context::RateLimitedSurface;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

#[derive(Debug, Clone)]
pub struct RateLimitState {
    pub policies: HashMap<RateLimitedSurface, SurfaceRateLimitPolicy>,
    pub windows: Arc<tokio::sync::Mutex<HashMap<RateLimitedSurface, SurfaceWindowState>>>,
}

#[derive(Debug, Clone)]
pub struct RateLimitMiddlewareState {
    pub state: RateLimitState,
    pub surface: RateLimitedSurface,
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

    pub async fn check(&self, surface: RateLimitedSurface) -> RateLimitDecision {
        let now = Instant::now();
        let policy = self
            .policies
            .get(&surface)
            .expect("covered surface policy must exist");
        let mut windows = self.windows.lock().await;
        let window = windows
            .entry(surface)
            .or_insert_with(|| SurfaceWindowState {
                window_started_at: now,
                request_count: 0,
            });
        evaluate_surface_limit(now, policy, window)
    }
}

pub fn evaluate_surface_limit(
    now: Instant,
    policy: &SurfaceRateLimitPolicy,
    window: &mut SurfaceWindowState,
) -> RateLimitDecision {
    let window_duration = Duration::from_secs(policy.window_seconds);

    if now.duration_since(window.window_started_at) >= window_duration {
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
    match state.state.check(state.surface).await {
        RateLimitDecision::Allow => next.run(request).await,
        RateLimitDecision::Reject {
            retry_after_seconds,
        } => match state.surface {
            RateLimitedSurface::AdminApi => admin_rate_limited_response(retry_after_seconds),
            RateLimitedSurface::GatewayModels | RateLimitedSurface::GatewayChatCompletions => {
                gateway_rate_limited_response(retry_after_seconds)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{evaluate_surface_limit, RateLimitDecision, RateLimitState, SurfaceWindowState};
    use crate::config::{RateLimitConfig, SurfaceRateLimitPolicy};
    use crate::transport::context::RateLimitedSurface;
    use std::time::{Duration, Instant};

    fn policy(max_requests: u32, window_seconds: u64) -> SurfaceRateLimitPolicy {
        SurfaceRateLimitPolicy {
            max_requests,
            window_seconds,
        }
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

    #[tokio::test(flavor = "current_thread")]
    async fn rate_limit_state_keeps_surface_budgets_independent() {
        let state = RateLimitState::new(&RateLimitConfig {
            api: policy(1, 60),
            v1_models: policy(1, 60),
            v1_chat_completions: policy(1, 60),
        });

        assert_eq!(
            state.check(RateLimitedSurface::AdminApi).await,
            RateLimitDecision::Allow
        );
        assert_eq!(
            state.check(RateLimitedSurface::AdminApi).await,
            RateLimitDecision::Reject {
                retry_after_seconds: 59,
            }
        );

        assert_eq!(
            state.check(RateLimitedSurface::GatewayModels).await,
            RateLimitDecision::Allow
        );
        assert_eq!(
            state
                .check(RateLimitedSurface::GatewayChatCompletions)
                .await,
            RateLimitDecision::Allow
        );
    }
}
