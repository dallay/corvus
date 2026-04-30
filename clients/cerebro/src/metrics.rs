use once_cell::sync::Lazy;
use prometheus::{
    register_counter, register_counter_vec, register_histogram_vec, Counter, CounterVec,
    HistogramVec,
};

pub static CEREBRO_REQUESTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "cerebro_requests_total",
        "Total number of MCP requests handled",
        &["method", "status"]
    )
    .unwrap()
});

pub static CEREBRO_TOOL_LATENCY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "cerebro_tool_latency_seconds",
        "Latency of MCP tool executions",
        &["tool", "status"]
    )
    .unwrap()
});

pub static CEREBRO_AUTH_FAILURES_TOTAL: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "cerebro_auth_failures_total",
        "Total number of authentication failures"
    )
    .unwrap()
});

pub static CEREBRO_READINESS_FAILURES_TOTAL: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "cerebro_readiness_failures_total",
        "Total number of readiness check failures"
    )
    .unwrap()
});

pub static CEREBRO_STORAGE_ERRORS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "cerebro_storage_errors_total",
        "Total number of storage operations that returned an error",
        &["operation"]
    )
    .unwrap()
});

/// Ensures all lazy static metrics are initialized and registered.
pub fn init() {
    Lazy::force(&CEREBRO_REQUESTS_TOTAL);
    Lazy::force(&CEREBRO_TOOL_LATENCY_SECONDS);
    Lazy::force(&CEREBRO_AUTH_FAILURES_TOTAL);
    Lazy::force(&CEREBRO_READINESS_FAILURES_TOTAL);
    Lazy::force(&CEREBRO_STORAGE_ERRORS_TOTAL);

    CEREBRO_REQUESTS_TOTAL.with_label_values(&["tools/call", "ok"]);
    CEREBRO_REQUESTS_TOTAL.with_label_values(&["tools/call", "error"]);
    CEREBRO_REQUESTS_TOTAL.with_label_values(&["tools/list", "ok"]);
    CEREBRO_REQUESTS_TOTAL.with_label_values(&["unknown", "error"]);
    CEREBRO_TOOL_LATENCY_SECONDS.with_label_values(&["unknown", "ok"]);
    CEREBRO_TOOL_LATENCY_SECONDS.with_label_values(&["unknown", "error"]);
    CEREBRO_STORAGE_ERRORS_TOTAL.with_label_values(&["unknown"]);
}
