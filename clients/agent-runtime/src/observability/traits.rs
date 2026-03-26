use std::time::Duration;

const SENSITIVE_PAYLOAD_MARKERS: [&str; 5] = ["password", "token", "secret", "api_key", "auth"];

// ── Image ingress telemetry ──────────────────────────────────

/// Outcome of an image ingress lifecycle event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageIngressOutcome {
    Admitted,
    Rejected,
    ProviderSent,
    ProviderError,
}

/// Metadata-only event for image ingress telemetry.
///
/// Never includes raw image bytes, channel URLs, tokens,
/// or base64 payloads — only routing and sizing metadata.
#[derive(Debug, Clone)]
pub struct ImageIngressEvent {
    pub channel: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub outcome: ImageIngressOutcome,
    pub reason: Option<String>,
    pub image_count: usize,
    pub mime_type: Option<String>,
    pub byte_len: Option<u64>,
}

pub fn redact_observer_payload(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let lower = trimmed.to_ascii_lowercase();
    if SENSITIVE_PAYLOAD_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return "***REDACTED***".to_string();
    }

    trimmed.to_string()
}

/// Events the observer can record
#[derive(Debug, Clone)]
pub enum ObserverEvent {
    AgentStart {
        provider: String,
        model: String,
    },
    /// A request is about to be sent to an LLM provider.
    ///
    /// This is emitted immediately before a provider call so observers can print
    /// user-facing progress without leaking prompt contents.
    LlmRequest {
        provider: String,
        model: String,
        messages_count: usize,
    },
    /// Result of a single LLM provider call.
    LlmResponse {
        provider: String,
        model: String,
        duration: Duration,
        success: bool,
        error_message: Option<String>,
    },
    AgentEnd {
        provider: String,
        model: String,
        duration: Duration,
        tokens_used: Option<u64>,
        cost_usd: Option<f64>,
    },
    /// A tool call is about to be executed.
    ToolCallStart {
        tool: String,
    },
    ToolCall {
        tool: String,
        duration: Duration,
        success: bool,
    },
    /// The agent produced a final answer for the current user message.
    TurnComplete,
    /// A code-specialist session completed with structured output.
    CodeSessionCompleted {
        session_id: String,
        status: String,
        summary: String,
        changed_files: Vec<String>,
        commands: Vec<String>,
        validations: Vec<String>,
        blockers: Vec<String>,
        pending_work: Vec<String>,
        delegated: bool,
    },
    ChannelMessage {
        channel: String,
        direction: String,
    },
    HeartbeatTick,
    Error {
        component: String,
        message: String,
    },
    /// Image ingress lifecycle event (metadata only).
    ImageIngress(ImageIngressEvent),
    /// Mission lifecycle started with deterministic mission id.
    MissionStarted {
        mission_id: String,
        checkpoint_count: u32,
        resume_from: Option<u32>,
    },
    /// Mission checkpoint progress update emitted on start/finish.
    MissionCheckpointProgress {
        mission_id: String,
        checkpoint_index: u32,
        status: String,
        duration: Duration,
    },
    /// Mission guardrail violation with secret-safe details.
    MissionGuardrailViolation {
        mission_id: String,
        checkpoint_index: Option<u32>,
        guardrail: String,
        termination_reason: String,
        detail: String,
    },
    MissionCompleted {
        mission_id: String,
        checkpoints_completed: u32,
        duration: Duration,
    },
    MissionTerminated {
        mission_id: String,
        checkpoint_index: Option<u32>,
        termination_reason: String,
        duration: Duration,
        rollback: bool,
    },
}

/// Numeric metrics
#[derive(Debug, Clone)]
pub enum ObserverMetric {
    RequestLatency(Duration),
    TokensUsed(u64),
    ActiveSessions(u64),
    QueueDepth(u64),
}

/// Core observability trait — implement for any backend
pub trait Observer: Send + Sync + 'static {
    /// Record a discrete event
    fn record_event(&self, event: &ObserverEvent);

    /// Record a numeric metric
    fn record_metric(&self, metric: &ObserverMetric);

    /// Record an image ingress lifecycle event (default: no-op).
    fn on_image_ingress(&self, _event: &ImageIngressEvent) {}

    /// Flush any buffered data (no-op for most backends)
    fn flush(&self) {}

    /// Human-readable name of this observer
    fn name(&self) -> &str;

    /// Downcast to `Any` for backend-specific operations
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::time::Duration;

    #[derive(Default)]
    struct DummyObserver {
        events: Mutex<u64>,
        metrics: Mutex<u64>,
    }

    impl Observer for DummyObserver {
        fn record_event(&self, _event: &ObserverEvent) {
            let mut guard = self.events.lock();
            *guard += 1;
        }

        fn record_metric(&self, _metric: &ObserverMetric) {
            let mut guard = self.metrics.lock();
            *guard += 1;
        }

        fn name(&self) -> &str {
            "dummy-observer"
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn observer_records_events_and_metrics() {
        let observer = DummyObserver::default();

        observer.record_event(&ObserverEvent::HeartbeatTick);
        observer.record_event(&ObserverEvent::Error {
            component: "test".into(),
            message: "boom".into(),
        });
        observer.record_metric(&ObserverMetric::TokensUsed(42));

        assert_eq!(*observer.events.lock(), 2);
        assert_eq!(*observer.metrics.lock(), 1);
    }

    #[test]
    fn observer_default_flush_and_as_any_work() {
        let observer = DummyObserver::default();

        observer.flush();
        assert_eq!(observer.name(), "dummy-observer");
        assert!(observer.as_any().downcast_ref::<DummyObserver>().is_some());
    }

    #[test]
    fn observer_event_and_metric_are_cloneable() {
        let event = ObserverEvent::ToolCall {
            tool: "shell".into(),
            duration: Duration::from_millis(10),
            success: true,
        };
        let metric = ObserverMetric::RequestLatency(Duration::from_millis(8));

        let cloned_event = event.clone();
        let cloned_metric = metric.clone();

        assert!(matches!(cloned_event, ObserverEvent::ToolCall { .. }));
        assert!(matches!(cloned_metric, ObserverMetric::RequestLatency(_)));
    }

    #[test]
    fn observer_event_code_session_completed_retains_fields() {
        let event = ObserverEvent::CodeSessionCompleted {
            session_id: "sess-123".into(),
            status: "completed".into(),
            summary: "Updated tests".into(),
            changed_files: vec!["src/lib.rs".into()],
            commands: vec!["cargo test".into()],
            validations: vec!["pass:cargo test".into()],
            blockers: vec![],
            pending_work: vec!["follow-up".into()],
            delegated: true,
        };

        let cloned = event.clone();
        match cloned {
            ObserverEvent::CodeSessionCompleted {
                session_id,
                status,
                summary,
                changed_files,
                commands,
                validations,
                blockers,
                pending_work,
                delegated,
            } => {
                assert_eq!(session_id, "sess-123");
                assert_eq!(status, "completed");
                assert_eq!(summary, "Updated tests");
                assert_eq!(changed_files, vec!["src/lib.rs".to_string()]);
                assert_eq!(commands, vec!["cargo test".to_string()]);
                assert_eq!(validations, vec!["pass:cargo test".to_string()]);
                assert!(blockers.is_empty());
                assert_eq!(pending_work, vec!["follow-up".to_string()]);
                assert!(delegated);
            }
            _ => panic!("unexpected event variant"),
        }
    }

    #[test]
    fn redact_observer_payload_masks_secret_markers() {
        assert_eq!(redact_observer_payload("token=abc123"), "***REDACTED***");
        assert_eq!(redact_observer_payload("password:123"), "***REDACTED***");
        assert_eq!(
            redact_observer_payload("checkpoint timeout"),
            "checkpoint timeout"
        );
    }

    // ── Image ingress telemetry (Task 4.4) ───────────────────

    #[test]
    fn image_ingress_event_construction_and_field_access() {
        let event = ImageIngressEvent {
            channel: "telegram".into(),
            provider: Some("gemini".into()),
            model: Some("gemini-2.0-flash".into()),
            outcome: ImageIngressOutcome::Admitted,
            reason: None,
            image_count: 1,
            mime_type: Some("image/jpeg".into()),
            byte_len: Some(204_800),
        };
        assert_eq!(event.channel, "telegram");
        assert_eq!(event.provider.as_deref(), Some("gemini"));
        assert_eq!(event.model.as_deref(), Some("gemini-2.0-flash"));
        assert_eq!(event.outcome, ImageIngressOutcome::Admitted);
        assert!(event.reason.is_none());
        assert_eq!(event.image_count, 1);
        assert_eq!(event.mime_type.as_deref(), Some("image/jpeg"));
        assert_eq!(event.byte_len, Some(204_800));
    }

    #[test]
    fn image_ingress_event_rejected_with_reason() {
        let event = ImageIngressEvent {
            channel: "whatsapp".into(),
            provider: None,
            model: None,
            outcome: ImageIngressOutcome::Rejected,
            reason: Some("disabled".into()),
            image_count: 1,
            mime_type: None,
            byte_len: None,
        };
        assert_eq!(event.outcome, ImageIngressOutcome::Rejected);
        assert_eq!(event.reason.as_deref(), Some("disabled"));
    }

    #[test]
    fn image_ingress_outcome_variants_are_distinct() {
        assert_ne!(ImageIngressOutcome::Admitted, ImageIngressOutcome::Rejected);
        assert_ne!(
            ImageIngressOutcome::ProviderSent,
            ImageIngressOutcome::ProviderError
        );
    }

    #[test]
    fn image_ingress_event_is_cloneable() {
        let event = ImageIngressEvent {
            channel: "telegram".into(),
            provider: None,
            model: None,
            outcome: ImageIngressOutcome::ProviderError,
            reason: Some("provider_error".into()),
            image_count: 1,
            mime_type: None,
            byte_len: None,
        };
        let cloned = event.clone();
        assert_eq!(cloned.channel, "telegram");
        assert_eq!(cloned.outcome, ImageIngressOutcome::ProviderError);
    }

    #[test]
    fn observer_event_image_ingress_variant_exists() {
        let event = ObserverEvent::ImageIngress(ImageIngressEvent {
            channel: "telegram".into(),
            provider: None,
            model: None,
            outcome: ImageIngressOutcome::Rejected,
            reason: Some("channel_not_allowed".into()),
            image_count: 1,
            mime_type: None,
            byte_len: None,
        });
        assert!(matches!(event, ObserverEvent::ImageIngress(_)));
    }

    #[test]
    fn observer_default_on_image_ingress_is_noop() {
        let observer = DummyObserver::default();
        let event = ImageIngressEvent {
            channel: "telegram".into(),
            provider: None,
            model: None,
            outcome: ImageIngressOutcome::Rejected,
            reason: Some("disabled".into()),
            image_count: 1,
            mime_type: None,
            byte_len: None,
        };
        // Must not panic
        observer.on_image_ingress(&event);
        // Event count unchanged — on_image_ingress is separate
        assert_eq!(*observer.events.lock(), 0);
    }
}
