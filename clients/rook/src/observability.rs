use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::histogram::{exponential_buckets, Histogram};
use prometheus_client::registry::Registry;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Observability {
    registry: Arc<Mutex<Registry>>,
    http_requests_total: Family<HttpRequestLabels, Counter>,
    http_request_duration_seconds: Family<HttpRequestLabels, Histogram>,
    rate_limit_rejections_total: Family<RateLimitLabels, Counter>,
    idempotency_outcomes_total: Family<IdempotencyLabels, Counter>,
    upstream_outcomes_total: Family<UpstreamOutcomeLabels, Counter>,
}

impl Observability {
    pub fn bootstrap() -> Result<Self, String> {
        let mut registry = Registry::default();

        let http_requests_total = Family::<HttpRequestLabels, Counter>::default();
        registry.register(
            "rook_http_requests_total",
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

        let rate_limit_rejections_total = Family::<RateLimitLabels, Counter>::default();
        registry.register(
            "rook_rate_limit_rejections_total",
            "Total rate-limit rejections partitioned by surface and endpoint.",
            rate_limit_rejections_total.clone(),
        );

        let idempotency_outcomes_total = Family::<IdempotencyLabels, Counter>::default();
        registry.register(
            "rook_idempotency_outcomes_total",
            "Total idempotency outcomes partitioned by surface and outcome.",
            idempotency_outcomes_total.clone(),
        );

        let upstream_outcomes_total = Family::<UpstreamOutcomeLabels, Counter>::default();
        registry.register(
            "rook_upstream_outcomes_total",
            "Total upstream request outcomes partitioned by vendor and outcome.",
            upstream_outcomes_total.clone(),
        );

        Ok(Self {
            registry: Arc::new(Mutex::new(registry)),
            http_requests_total,
            http_request_duration_seconds,
            rate_limit_rejections_total,
            idempotency_outcomes_total,
            upstream_outcomes_total,
        })
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

    pub fn rate_limit_rejections_total(&self) -> RateLimitRejectionsHandle {
        RateLimitRejectionsHandle {
            family: self.rate_limit_rejections_total.clone(),
        }
    }

    pub fn idempotency_outcomes_total(&self) -> IdempotencyOutcomesHandle {
        IdempotencyOutcomesHandle {
            family: self.idempotency_outcomes_total.clone(),
        }
    }

    pub fn upstream_outcomes_total(&self) -> UpstreamOutcomesHandle {
        UpstreamOutcomesHandle {
            family: self.upstream_outcomes_total.clone(),
        }
    }
}

#[derive(Clone)]
pub struct HttpRequestsTotalHandle {
    family: Family<HttpRequestLabels, Counter>,
}

impl HttpRequestsTotalHandle {
    pub fn inc(&self, surface: &str, endpoint: &str, status_class: &str) {
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
    pub fn observe(&self, surface: &str, endpoint: &str, status_class: &str, seconds: f64) {
        self.family
            .get_or_create(&HttpRequestLabels::new(surface, endpoint, status_class))
            .observe(seconds);
    }
}

#[derive(Clone)]
pub struct RateLimitRejectionsHandle {
    family: Family<RateLimitLabels, Counter>,
}

impl RateLimitRejectionsHandle {
    pub fn inc(&self, surface: &str, endpoint: &str) {
        self.family
            .get_or_create(&RateLimitLabels::new(surface, endpoint))
            .inc();
    }
}

#[derive(Clone)]
pub struct IdempotencyOutcomesHandle {
    family: Family<IdempotencyLabels, Counter>,
}

impl IdempotencyOutcomesHandle {
    pub fn inc(&self, surface: &str, outcome: &str) {
        self.family
            .get_or_create(&IdempotencyLabels::new(surface, outcome))
            .inc();
    }
}

#[derive(Clone)]
pub struct UpstreamOutcomesHandle {
    family: Family<UpstreamOutcomeLabels, Counter>,
}

impl UpstreamOutcomesHandle {
    pub fn inc(&self, vendor: &str, outcome: &str) {
        self.family
            .get_or_create(&UpstreamOutcomeLabels::new(vendor, outcome))
            .inc();
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct HttpRequestLabels {
    surface: String,
    endpoint: String,
    status_class: String,
}

impl HttpRequestLabels {
    fn new(surface: &str, endpoint: &str, status_class: &str) -> Self {
        Self {
            surface: surface.to_string(),
            endpoint: endpoint.to_string(),
            status_class: status_class.to_string(),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct RateLimitLabels {
    surface: String,
    endpoint: String,
}

impl RateLimitLabels {
    fn new(surface: &str, endpoint: &str) -> Self {
        Self {
            surface: surface.to_string(),
            endpoint: endpoint.to_string(),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct IdempotencyLabels {
    surface: String,
    outcome: String,
}

impl IdempotencyLabels {
    fn new(surface: &str, outcome: &str) -> Self {
        Self {
            surface: surface.to_string(),
            outcome: outcome.to_string(),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct UpstreamOutcomeLabels {
    vendor: String,
    outcome: String,
}

impl UpstreamOutcomeLabels {
    fn new(vendor: &str, outcome: &str) -> Self {
        Self {
            vendor: vendor.to_string(),
            outcome: outcome.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_registers_phase_one_metric_families() {
        let metrics = Observability::bootstrap().expect("observability bootstrap should succeed");
        let rendered = metrics
            .render_prometheus()
            .expect("metrics should render in prometheus text format");

        assert!(rendered.contains("# TYPE rook_http_requests_total counter"));
        assert!(rendered.contains("# TYPE rook_http_request_duration_seconds histogram"));
        assert!(rendered.contains("# TYPE rook_rate_limit_rejections_total counter"));
        assert!(rendered.contains("# TYPE rook_idempotency_outcomes_total counter"));
        assert!(rendered.contains("# TYPE rook_upstream_outcomes_total counter"));
    }

    #[test]
    fn bootstrap_starts_with_empty_phase_one_series() {
        let metrics = Observability::bootstrap().expect("observability bootstrap should succeed");
        let rendered = metrics
            .render_prometheus()
            .expect("metrics should render in prometheus text format");

        assert!(!rendered.contains("rook_http_requests_total{"));
        assert!(!rendered.contains("rook_rate_limit_rejections_total{"));
        assert!(!rendered.contains("rook_idempotency_outcomes_total{"));
        assert!(!rendered.contains("rook_upstream_outcomes_total{"));
    }

    #[test]
    fn metric_handles_record_samples_into_registry_output() {
        let metrics = Observability::bootstrap().expect("observability bootstrap should succeed");

        metrics.http_requests_total().inc("admin_api", "/api/health", "2xx");
        metrics
            .rate_limit_rejections_total()
            .inc("admin_api", "/api/health");
        metrics
            .idempotency_outcomes_total()
            .inc("chat_completions", "replay");
        metrics
            .upstream_outcomes_total()
            .inc("open_ai", "success");
        metrics
            .http_request_duration_seconds()
            .observe("gateway_models", "/v1/models", "2xx", 0.125);

        let rendered = metrics
            .render_prometheus()
            .expect("metrics should render in prometheus text format");

        assert!(rendered.contains("rook_http_requests_total_total{surface=\"admin_api\",endpoint=\"/api/health\",status_class=\"2xx\"} 1"));
        assert!(rendered.contains("rook_rate_limit_rejections_total_total{surface=\"admin_api\",endpoint=\"/api/health\"} 1"));
        assert!(rendered.contains("rook_idempotency_outcomes_total_total{surface=\"chat_completions\",outcome=\"replay\"} 1"));
        assert!(rendered.contains("rook_upstream_outcomes_total_total{vendor=\"open_ai\",outcome=\"success\"} 1"));
        assert!(rendered.contains("rook_http_request_duration_seconds_sum{surface=\"gateway_models\",endpoint=\"/v1/models\",status_class=\"2xx\"} 0.125"));
        assert!(rendered.contains("rook_http_request_duration_seconds_count{surface=\"gateway_models\",endpoint=\"/v1/models\",status_class=\"2xx\"} 1"));
    }
}
