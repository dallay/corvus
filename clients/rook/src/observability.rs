use axum::http::StatusCode;
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::histogram::{exponential_buckets, Histogram};
use prometheus_client::registry::Registry;
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use crate::domain::ProviderVendor;
use crate::transport::context::{RateLimitedSurface, RouteSurface};

const UNMATCHED_ENDPOINT: &str = "unmatched";
const UNLABELED_ACCOUNT: &str = "unlabeled";
const UNROUTED_MODEL: &str = "unrouted";
const MAX_LABEL_LEN: usize = 64;

#[derive(Debug, Clone)]
pub struct Observability {
    registry: Arc<Mutex<Registry>>,
    http_requests_total: Family<HttpRequestLabels, Counter>,
    http_request_duration_seconds: Family<HttpRequestLabels, Histogram>,
    rate_limit_outcomes_total: Family<RateLimitOutcomeLabels, Counter>,
    idempotency_outcomes_total: Family<IdempotencyLabels, Counter>,
    upstream_failures_total: Family<UpstreamFailureLabels, Counter>,
}

impl Observability {
    pub fn bootstrap() -> Self {
        let mut registry = Registry::default();

        let http_requests_total = Family::<HttpRequestLabels, Counter>::default();
        registry.register(
            "rook_http_requests",
            "Total HTTP requests partitioned by surface, endpoint, and status class.",
            http_requests_total.clone(),
        );

        let http_request_duration_seconds =
            Family::<HttpRequestLabels, Histogram>::new_with_constructor(|| {
                Histogram::new(exponential_buckets(0.005, 2.0, 16))
            });
        registry.register(
            "rook_http_request_duration_seconds",
            "HTTP request duration in seconds partitioned by surface, endpoint, and status class.",
            http_request_duration_seconds.clone(),
        );

        let rate_limit_outcomes_total = Family::<RateLimitOutcomeLabels, Counter>::default();
        registry.register(
            "rook_rate_limit_outcomes",
            "Total rate-limit outcomes partitioned by surface, endpoint, and outcome.",
            rate_limit_outcomes_total.clone(),
        );

        let idempotency_outcomes_total = Family::<IdempotencyLabels, Counter>::default();
        registry.register(
            "rook_idempotency_outcomes",
            "Total idempotency outcomes partitioned by surface and outcome.",
            idempotency_outcomes_total.clone(),
        );

        let upstream_failures_total = Family::<UpstreamFailureLabels, Counter>::default();
        registry.register(
            "rook_upstream_failures",
            "Total upstream failure outcomes partitioned by vendor, account, model, and outcome.",
            upstream_failures_total.clone(),
        );

        Self {
            registry: Arc::new(Mutex::new(registry)),
            http_requests_total,
            http_request_duration_seconds,
            rate_limit_outcomes_total,
            idempotency_outcomes_total,
            upstream_failures_total,
        }
    }

    pub fn render_prometheus(&self) -> Result<String, String> {
        let registry = self
            .registry
            .lock()
            .map_err(|_| "prometheus registry lock poisoned".to_string())?;
        let mut rendered = String::new();
        encode(&mut rendered, &registry).map_err(|error| error.to_string())?;
        Ok(rendered)
    }

    pub fn http_requests_total(&self) -> HttpRequestsTotalHandle {
        HttpRequestsTotalHandle {
            family: self.http_requests_total.clone(),
        }
    }

    pub fn http_request_duration_seconds(&self) -> HttpRequestDurationHandle {
        HttpRequestDurationHandle {
            family: self.http_request_duration_seconds.clone(),
        }
    }

    pub fn rate_limit_outcomes_total(&self) -> RateLimitOutcomesHandle {
        RateLimitOutcomesHandle {
            family: self.rate_limit_outcomes_total.clone(),
        }
    }

    pub fn idempotency_outcomes_total(&self) -> IdempotencyOutcomesHandle {
        IdempotencyOutcomesHandle {
            family: self.idempotency_outcomes_total.clone(),
        }
    }

    pub fn upstream_failures_total(&self) -> UpstreamFailuresHandle {
        UpstreamFailuresHandle {
            family: self.upstream_failures_total.clone(),
        }
    }
}

pub fn normalize_http_surface(surface: RouteSurface) -> &'static str {
    match surface {
        RouteSurface::AdminApi => "admin_api",
        RouteSurface::GatewayV1 => "gateway_v1",
    }
}

pub fn normalize_rate_limit_surface(surface: RateLimitedSurface) -> &'static str {
    match surface {
        RateLimitedSurface::AdminApi => "admin_api",
        RateLimitedSurface::GatewayModels | RateLimitedSurface::GatewayChatCompletions => {
            "gateway_v1"
        }
    }
}

pub fn normalize_status_class(status: StatusCode) -> &'static str {
    match status.as_u16() {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        _ => "5xx",
    }
}

pub fn normalize_http_endpoint(
    surface: RouteSurface,
    matched_path: Option<&str>,
) -> Cow<'static, str> {
    normalize_surface_endpoint(normalize_http_surface(surface), matched_path)
}

pub fn normalize_rate_limit_endpoint(
    surface: RateLimitedSurface,
    matched_path: Option<&str>,
) -> Cow<'static, str> {
    normalize_surface_endpoint(normalize_rate_limit_surface(surface), matched_path)
}

pub fn normalize_surface_endpoint(surface: &str, matched_path: Option<&str>) -> Cow<'static, str> {
    let Some(path) = matched_path else {
        return Cow::Borrowed(UNMATCHED_ENDPOINT);
    };

    if !path.starts_with('/') {
        return Cow::Borrowed(UNMATCHED_ENDPOINT);
    }

    if path.starts_with("/api/") || path == "/api" || path.starts_with("/v1/") || path == "/v1" {
        return Cow::Owned(path.to_string());
    }

    let prefix = match surface {
        "admin_api" => "/api",
        "gateway_v1" | "gateway_models" | "gateway_chat_completions" => "/v1",
        _ => return Cow::Borrowed(UNMATCHED_ENDPOINT),
    };

    if path == "/" {
        return Cow::Owned(prefix.to_string());
    }

    Cow::Owned(format!("{prefix}{path}"))
}

pub fn normalize_vendor_label(vendor: &ProviderVendor) -> &'static str {
    match vendor {
        ProviderVendor::OpenAi => "open_ai",
        ProviderVendor::Anthropic => "anthropic",
        ProviderVendor::Google => "google",
        ProviderVendor::OpenRouter => "open_router",
        ProviderVendor::DeepSeek => "deep_seek",
        ProviderVendor::Other(_) => "other",
    }
}

pub fn normalize_account_label(display_name: Option<&str>) -> Cow<'static, str> {
    normalize_bounded_label(display_name, UNLABELED_ACCOUNT)
}

pub fn normalize_model_label(model: Option<&str>) -> Cow<'static, str> {
    normalize_bounded_label(model, UNROUTED_MODEL)
}

fn looks_secret_like(value: &str) -> bool {
    let lowercase = value.to_ascii_lowercase();
    lowercase.contains("bearer ")
        || lowercase.starts_with("sk-")
        || lowercase.contains(" sk-")
        || lowercase.contains("api_key")
        || lowercase.contains("api-key")
        || lowercase.starts_with("token ")
        || lowercase.contains(" token ")
}

fn normalize_bounded_label(value: Option<&str>, fallback: &'static str) -> Cow<'static, str> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Cow::Borrowed(fallback);
    };

    if looks_secret_like(value) {
        return Cow::Borrowed(fallback);
    }

    let mut normalized = String::with_capacity(value.len());
    let mut last_was_separator = false;

    for ch in value.chars() {
        let normalized_ch = match ch {
            'a'..='z' | '0'..='9' => Some(ch),
            'A'..='Z' => Some(ch.to_ascii_lowercase()),
            '-' | '_' | '.' => Some(ch),
            ' ' | '/' | ':' => Some('_'),
            _ => None,
        };

        let Some(ch) = normalized_ch else {
            continue;
        };

        if matches!(ch, '_' | '-' | '.') {
            if normalized.is_empty() || last_was_separator {
                continue;
            }
            last_was_separator = true;
        } else {
            last_was_separator = false;
        }

        normalized.push(ch);
        if normalized.len() > MAX_LABEL_LEN {
            return Cow::Borrowed(fallback);
        }
    }

    let trimmed = normalized.trim_matches(|ch| matches!(ch, '_' | '-' | '.'));
    if trimmed.is_empty() {
        Cow::Borrowed(fallback)
    } else {
        Cow::Owned(trimmed.to_string())
    }
}

#[derive(Clone)]
pub struct HttpRequestsTotalHandle {
    family: Family<HttpRequestLabels, Counter>,
}

impl HttpRequestsTotalHandle {
    pub fn inc(
        &self,
        surface: impl Into<Cow<'static, str>>,
        endpoint: impl Into<Cow<'static, str>>,
        status_class: impl Into<Cow<'static, str>>,
    ) {
        self.family
            .get_or_create(&HttpRequestLabels::new(surface, endpoint, status_class))
            .inc();
    }
}

#[derive(Clone)]
pub struct HttpRequestDurationHandle {
    family: Family<HttpRequestLabels, Histogram>,
}

impl HttpRequestDurationHandle {
    pub fn observe(
        &self,
        surface: impl Into<Cow<'static, str>>,
        endpoint: impl Into<Cow<'static, str>>,
        status_class: impl Into<Cow<'static, str>>,
        seconds: f64,
    ) {
        self.family
            .get_or_create(&HttpRequestLabels::new(surface, endpoint, status_class))
            .observe(seconds);
    }
}

#[derive(Clone)]
pub struct RateLimitOutcomesHandle {
    family: Family<RateLimitOutcomeLabels, Counter>,
}

impl RateLimitOutcomesHandle {
    pub fn inc(
        &self,
        surface: impl Into<Cow<'static, str>>,
        endpoint: impl Into<Cow<'static, str>>,
        outcome: impl Into<Cow<'static, str>>,
    ) {
        self.family
            .get_or_create(&RateLimitOutcomeLabels::new(surface, endpoint, outcome))
            .inc();
    }
}

#[derive(Clone)]
pub struct IdempotencyOutcomesHandle {
    family: Family<IdempotencyLabels, Counter>,
}

impl IdempotencyOutcomesHandle {
    pub fn inc(
        &self,
        surface: impl Into<Cow<'static, str>>,
        outcome: impl Into<Cow<'static, str>>,
    ) {
        self.family
            .get_or_create(&IdempotencyLabels::new(surface, outcome))
            .inc();
    }
}

#[derive(Clone)]
pub struct UpstreamFailuresHandle {
    family: Family<UpstreamFailureLabels, Counter>,
}

impl UpstreamFailuresHandle {
    pub fn inc(
        &self,
        vendor: impl Into<Cow<'static, str>>,
        account: impl Into<Cow<'static, str>>,
        model: impl Into<Cow<'static, str>>,
        outcome: impl Into<Cow<'static, str>>,
    ) {
        self.family
            .get_or_create(&UpstreamFailureLabels::new(vendor, account, model, outcome))
            .inc();
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct HttpRequestLabels {
    surface: Cow<'static, str>,
    endpoint: Cow<'static, str>,
    status_class: Cow<'static, str>,
}

impl HttpRequestLabels {
    fn new(
        surface: impl Into<Cow<'static, str>>,
        endpoint: impl Into<Cow<'static, str>>,
        status_class: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            surface: surface.into(),
            endpoint: endpoint.into(),
            status_class: status_class.into(),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct RateLimitOutcomeLabels {
    surface: Cow<'static, str>,
    endpoint: Cow<'static, str>,
    outcome: Cow<'static, str>,
}

impl RateLimitOutcomeLabels {
    fn new(
        surface: impl Into<Cow<'static, str>>,
        endpoint: impl Into<Cow<'static, str>>,
        outcome: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            surface: surface.into(),
            endpoint: endpoint.into(),
            outcome: outcome.into(),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct IdempotencyLabels {
    surface: Cow<'static, str>,
    outcome: Cow<'static, str>,
}

impl IdempotencyLabels {
    fn new(surface: impl Into<Cow<'static, str>>, outcome: impl Into<Cow<'static, str>>) -> Self {
        Self {
            surface: surface.into(),
            outcome: outcome.into(),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct UpstreamFailureLabels {
    vendor: Cow<'static, str>,
    account: Cow<'static, str>,
    model: Cow<'static, str>,
    outcome: Cow<'static, str>,
}

impl UpstreamFailureLabels {
    fn new(
        vendor: impl Into<Cow<'static, str>>,
        account: impl Into<Cow<'static, str>>,
        model: impl Into<Cow<'static, str>>,
        outcome: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            vendor: vendor.into(),
            account: account.into(),
            model: model.into(),
            outcome: outcome.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_registers_production_metric_families() {
        let metrics = Observability::bootstrap();
        let rendered = metrics
            .render_prometheus()
            .expect("metrics should render in prometheus text format");

        assert!(rendered.contains("# TYPE rook_http_requests counter"));
        assert!(rendered.contains("# TYPE rook_http_request_duration_seconds histogram"));
        assert!(rendered.contains("# TYPE rook_rate_limit_outcomes counter"));
        assert!(rendered.contains("# TYPE rook_idempotency_outcomes counter"));
        assert!(rendered.contains("# TYPE rook_upstream_failures counter"));
        assert!(!rendered.contains("_total_total"));
    }

    #[test]
    fn bootstrap_starts_with_empty_production_series() {
        let metrics = Observability::bootstrap();
        let rendered = metrics
            .render_prometheus()
            .expect("metrics should render in prometheus text format");

        assert!(!rendered.contains("rook_http_requests_total{"));
        assert!(!rendered.contains("rook_rate_limit_outcomes_total{"));
        assert!(!rendered.contains("rook_idempotency_outcomes_total{"));
        assert!(!rendered.contains("rook_upstream_failures_total{"));
    }

    #[test]
    fn metric_handles_record_samples_into_registry_output() {
        let metrics = Observability::bootstrap();

        metrics
            .http_requests_total()
            .inc("admin_api", "/api/health", "2xx");
        metrics
            .rate_limit_outcomes_total()
            .inc("gateway_v1", "/v1/chat/completions", "reject");
        metrics
            .idempotency_outcomes_total()
            .inc("gateway_chat_completions", "unavailable");
        metrics
            .upstream_failures_total()
            .inc("open_ai", "primary_account", "gpt-4o", "http_error");
        metrics
            .http_request_duration_seconds()
            .observe("gateway_v1", "/v1/models", "2xx", 0.125);

        let rendered = metrics
            .render_prometheus()
            .expect("metrics should render in prometheus text format");

        assert!(rendered.contains(
            "rook_http_requests_total{surface=\"admin_api\",endpoint=\"/api/health\",status_class=\"2xx\"} 1"
        ));
        assert!(rendered.contains(
            "rook_rate_limit_outcomes_total{surface=\"gateway_v1\",endpoint=\"/v1/chat/completions\",outcome=\"reject\"} 1"
        ));
        assert!(rendered.contains(
            "rook_idempotency_outcomes_total{surface=\"gateway_chat_completions\",outcome=\"unavailable\"} 1"
        ));
        assert!(rendered.contains(
            "rook_upstream_failures_total{vendor=\"open_ai\",account=\"primary_account\",model=\"gpt-4o\",outcome=\"http_error\"} 1"
        ));
        assert!(rendered.contains(
            "rook_http_request_duration_seconds_sum{surface=\"gateway_v1\",endpoint=\"/v1/models\",status_class=\"2xx\"} 0.125"
        ));
        assert!(rendered.contains(
            "rook_http_request_duration_seconds_count{surface=\"gateway_v1\",endpoint=\"/v1/models\",status_class=\"2xx\"} 1"
        ));
    }

    #[test]
    fn normalization_helpers_keep_labels_bounded_and_secret_safe() {
        assert_eq!(normalize_http_surface(RouteSurface::AdminApi), "admin_api");
        assert_eq!(
            normalize_http_surface(RouteSurface::GatewayV1),
            "gateway_v1"
        );
        assert_eq!(
            normalize_rate_limit_surface(RateLimitedSurface::GatewayChatCompletions),
            "gateway_v1"
        );
        assert_eq!(normalize_status_class(StatusCode::OK), "2xx");
        assert_eq!(normalize_status_class(StatusCode::BAD_GATEWAY), "5xx");
        assert_eq!(
            normalize_http_endpoint(RouteSurface::GatewayV1, Some("/chat/completions")),
            "/v1/chat/completions"
        );
        assert_eq!(
            normalize_http_endpoint(RouteSurface::AdminApi, Some("/accounts/{account_id}")),
            "/api/accounts/{account_id}"
        );
        assert_eq!(
            normalize_surface_endpoint("gateway_v1", Some("accounts/123")),
            UNMATCHED_ENDPOINT
        );
        assert_eq!(normalize_vendor_label(&ProviderVendor::OpenAi), "open_ai");
        assert_eq!(
            normalize_account_label(Some("Primary Account / prod")),
            "primary_account_prod"
        );
        assert_eq!(
            normalize_account_label(Some("Bearer sk-secret")),
            UNLABELED_ACCOUNT
        );
        assert_eq!(
            normalize_account_label(Some("sk-secret-value")),
            UNLABELED_ACCOUNT
        );
        assert_eq!(normalize_account_label(Some("@@@")), UNLABELED_ACCOUNT);
        assert_eq!(normalize_model_label(Some("gpt-4o")), "gpt-4o");
        assert_eq!(
            normalize_model_label(Some("Bearer sk-secret")),
            UNROUTED_MODEL
        );
        assert_eq!(normalize_model_label(Some(" ")), UNROUTED_MODEL);
        assert_eq!(
            normalize_model_label(Some(&"x".repeat(MAX_LABEL_LEN + 1))),
            UNROUTED_MODEL
        );
    }
}
