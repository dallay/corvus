use crate::cost::{BudgetState, CostOverrideScope, UsagePeriod};
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

/// Closed set of reasons for image ingress rejection/failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageIngressReason {
    Disabled,
    ChannelNotAllowed,
    MissingVisionRoute,
    RouteNotImageCapable,
    FetchFailed,
    MimeRejected,
    Oversize,
    TooManyImages,
    ProviderError,
    ChannelNotSupported,
}

impl std::fmt::Display for ImageIngressReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = match self {
            Self::Disabled => "disabled",
            Self::ChannelNotAllowed => "channel_not_allowed",
            Self::MissingVisionRoute => "missing_vision_route",
            Self::RouteNotImageCapable => "route_not_image_capable",
            Self::FetchFailed => "fetch_failed",
            Self::MimeRejected => "mime_rejected",
            Self::Oversize => "oversize",
            Self::TooManyImages => "too_many_images",
            Self::ProviderError => "provider_error",
            Self::ChannelNotSupported => "channel_not_supported",
        };
        f.write_str(code)
    }
}

/// Metadata-only event for image ingress telemetry.
///
/// Never includes raw image bytes, channel URLs, tokens,
/// or base64 payloads — only routing and sizing metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageIngressImageMeta {
    pub mime_type: String,
    pub byte_len: u64,
}

#[derive(Debug, Clone)]
pub struct ImageIngressEvent {
    pub channel: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub outcome: ImageIngressOutcome,
    pub reason: Option<ImageIngressReason>,
    pub image_count: usize,
    pub max_images_per_turn: Option<usize>,
    pub images: Vec<ImageIngressImageMeta>,
    pub total_byte_len: Option<u64>,
    pub mime_type: Option<String>,
    pub byte_len: Option<u64>,
}

// ── Audio ingress telemetry ──────────────────────────────────

/// Outcome of an audio ingress lifecycle event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioIngressOutcome {
    Admitted,
    Rejected,
}

/// Closed set of reasons for audio ingress rejection/failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioIngressReason {
    Disabled,
    ChannelNotAllowed,
    FetchFailed,
    MimeRejected,
    Oversize,
    TooLong,
    Corrupted,
    TranscriptionFailed,
    NoSpeechDetected,
    TranscriberUnavailable,
    MultipleAudioParts,
    SystemError,
}

impl std::fmt::Display for AudioIngressReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = match self {
            Self::Disabled => "disabled",
            Self::ChannelNotAllowed => "channel_not_allowed",
            Self::FetchFailed => "fetch_failed",
            Self::MimeRejected => "mime_rejected",
            Self::Oversize => "oversize",
            Self::TooLong => "too_long",
            Self::Corrupted => "corrupted",
            Self::TranscriptionFailed => "transcription_failed",
            Self::NoSpeechDetected => "no_speech_detected",
            Self::TranscriberUnavailable => "transcriber_unavailable",
            Self::MultipleAudioParts => "multiple_audio_parts",
            Self::SystemError => "system_error",
        };
        f.write_str(code)
    }
}

/// Metadata-only event for audio ingress telemetry.
///
/// Never includes raw audio bytes, channel URLs, tokens,
/// or base64 payloads — only routing and sizing metadata.
#[derive(Debug, Clone)]
pub struct AudioIngressEvent {
    pub channel: String,
    pub outcome: AudioIngressOutcome,
    pub reason: Option<AudioIngressReason>,
    pub mime_type: Option<String>,
    pub byte_len: Option<u64>,
    pub duration_secs: Option<f64>,
    pub transcription_duration_ms: Option<u64>,
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

pub fn redact_optional_observer_payload(value: Option<&str>) -> Option<String> {
    value.map(redact_observer_payload)
}

pub fn usage_period_label(period: UsagePeriod) -> &'static str {
    match period {
        UsagePeriod::Session => "session",
        UsagePeriod::Day => "day",
        UsagePeriod::Month => "month",
        UsagePeriod::Mission => "mission",
    }
}

pub fn budget_state_label(state: BudgetState) -> &'static str {
    match state {
        BudgetState::Allowed => "allowed",
        BudgetState::Warning => "warning",
        BudgetState::Exceeded => "exceeded",
    }
}

pub fn cost_override_scope_label(scope: CostOverrideScope) -> &'static str {
    match scope {
        CostOverrideScope::NextRequest => "next_request",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetThresholdEvent {
    pub budget_state: BudgetState,
    pub period: UsagePeriod,
    pub current_usd: f64,
    pub projected_usd: f64,
    pub limit_usd: f64,
    pub percent_used: f64,
    pub session_id: String,
    pub surface: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetOverrideAction {
    Granted,
    Consumed,
}

impl BudgetOverrideAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Consumed => "consumed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetOverrideEvent {
    pub action: BudgetOverrideAction,
    pub actor: String,
    pub scope: CostOverrideScope,
    pub reason: Option<String>,
    pub session_id: Option<String>,
    pub previous_state: BudgetState,
    pub period: Option<UsagePeriod>,
    pub override_id: Option<String>,
    pub surface: Option<String>,
}

impl BudgetOverrideEvent {
    pub fn redacted_actor(&self) -> String {
        redact_observer_payload(&self.actor)
    }

    pub fn redacted_reason(&self) -> Option<String> {
        redact_optional_observer_payload(self.reason.as_deref())
    }
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
    BudgetWarning(BudgetThresholdEvent),
    BudgetExceeded(BudgetThresholdEvent),
    BudgetOverride(BudgetOverrideEvent),
    /// Image ingress lifecycle event (metadata only).
    ImageIngress(ImageIngressEvent),
    /// Audio ingress lifecycle event (metadata only).
    AudioIngress(AudioIngressEvent),
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

    /// Record an image ingress lifecycle event.
    ///
    /// Default: forwards to `record_event` as `ObserverEvent::ImageIngress`.
    fn on_image_ingress(&self, event: &ImageIngressEvent) {
        self.record_event(&ObserverEvent::ImageIngress(event.clone()));
    }

    /// Record an audio ingress lifecycle event.
    ///
    /// Default: forwards to `record_event` as `ObserverEvent::AudioIngress`.
    fn on_audio_ingress(&self, event: &AudioIngressEvent) {
        self.record_event(&ObserverEvent::AudioIngress(event.clone()));
    }

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
    use crate::cost::{BudgetState, CostOverrideScope, UsagePeriod};
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

    #[test]
    fn budget_override_event_redacts_sensitive_actor_and_reason() {
        let event = BudgetOverrideEvent {
            action: BudgetOverrideAction::Granted,
            actor: "paired-admin-token".into(),
            scope: CostOverrideScope::NextRequest,
            reason: Some("token=super-secret".into()),
            session_id: Some("sess-123".into()),
            previous_state: BudgetState::Exceeded,
            period: Some(UsagePeriod::Day),
            override_id: Some("ovr-123".into()),
            surface: Some("gateway_admin".into()),
        };

        assert_eq!(event.redacted_actor(), "***REDACTED***");
        assert_eq!(event.redacted_reason().as_deref(), Some("***REDACTED***"));
    }

    #[test]
    fn observer_event_budget_variants_exist() {
        let warning = ObserverEvent::BudgetWarning(BudgetThresholdEvent {
            budget_state: BudgetState::Warning,
            period: UsagePeriod::Day,
            current_usd: 8.2,
            projected_usd: 8.2,
            limit_usd: 10.0,
            percent_used: 82.0,
            session_id: "sess-123".into(),
            surface: Some("agent_loop".into()),
        });
        let exceeded = ObserverEvent::BudgetExceeded(BudgetThresholdEvent {
            budget_state: BudgetState::Exceeded,
            period: UsagePeriod::Day,
            current_usd: 10.2,
            projected_usd: 10.3,
            limit_usd: 10.0,
            percent_used: 103.0,
            session_id: "sess-123".into(),
            surface: Some("agent_loop".into()),
        });
        let override_event = ObserverEvent::BudgetOverride(BudgetOverrideEvent {
            action: BudgetOverrideAction::Consumed,
            actor: "cli-agent".into(),
            scope: CostOverrideScope::NextRequest,
            reason: Some("incident mitigation".into()),
            session_id: Some("sess-123".into()),
            previous_state: BudgetState::Exceeded,
            period: Some(UsagePeriod::Day),
            override_id: Some("ovr-123".into()),
            surface: Some("cli".into()),
        });

        assert!(matches!(warning, ObserverEvent::BudgetWarning(_)));
        assert!(matches!(exceeded, ObserverEvent::BudgetExceeded(_)));
        assert!(matches!(override_event, ObserverEvent::BudgetOverride(_)));
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
            image_count: 2,
            max_images_per_turn: None,
            images: vec![
                ImageIngressImageMeta {
                    mime_type: "image/jpeg".into(),
                    byte_len: 204_800,
                },
                ImageIngressImageMeta {
                    mime_type: "image/png".into(),
                    byte_len: 102_400,
                },
            ],
            total_byte_len: Some(307_200),
            mime_type: None,
            byte_len: None,
        };
        assert_eq!(event.channel, "telegram");
        assert_eq!(event.provider.as_deref(), Some("gemini"));
        assert_eq!(event.model.as_deref(), Some("gemini-2.0-flash"));
        assert_eq!(event.outcome, ImageIngressOutcome::Admitted);
        assert!(event.reason.is_none());
        assert_eq!(event.image_count, 2);
        assert_eq!(event.images.len(), 2);
        assert_eq!(event.images[1].mime_type, "image/png");
        assert_eq!(event.total_byte_len, Some(307_200));
        assert!(event.mime_type.is_none());
        assert!(event.byte_len.is_none());
    }

    #[test]
    fn image_ingress_event_rejected_with_reason() {
        let event = ImageIngressEvent {
            channel: "whatsapp".into(),
            provider: None,
            model: None,
            outcome: ImageIngressOutcome::Rejected,
            reason: Some(ImageIngressReason::Disabled),
            image_count: 4,
            max_images_per_turn: None,
            images: Vec::new(),
            total_byte_len: None,
            mime_type: None,
            byte_len: None,
        };
        assert_eq!(event.outcome, ImageIngressOutcome::Rejected);
        assert_eq!(event.reason, Some(ImageIngressReason::Disabled));
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
            reason: Some(ImageIngressReason::ProviderError),
            image_count: 1,
            max_images_per_turn: None,
            images: vec![ImageIngressImageMeta {
                mime_type: "image/webp".into(),
                byte_len: 333,
            }],
            total_byte_len: Some(333),
            mime_type: Some("image/webp".into()),
            byte_len: Some(333),
        };
        let cloned = event.clone();
        assert_eq!(cloned.channel, "telegram");
        assert_eq!(cloned.outcome, ImageIngressOutcome::ProviderError);
        assert_eq!(cloned.images.len(), 1);
        assert_eq!(cloned.mime_type.as_deref(), Some("image/webp"));
        assert_eq!(cloned.byte_len, Some(333));
    }

    #[test]
    fn observer_event_image_ingress_variant_exists() {
        let event = ObserverEvent::ImageIngress(ImageIngressEvent {
            channel: "telegram".into(),
            provider: None,
            model: None,
            outcome: ImageIngressOutcome::Rejected,
            reason: Some(ImageIngressReason::ChannelNotAllowed),
            image_count: 1,
            max_images_per_turn: None,
            images: Vec::new(),
            total_byte_len: None,
            mime_type: None,
            byte_len: None,
        });
        assert!(matches!(event, ObserverEvent::ImageIngress(_)));
    }

    // ── Audio ingress telemetry tests (Task 1.4 — audio-input-support) ──

    #[test]
    fn audio_ingress_outcome_variants_are_distinct() {
        assert_ne!(AudioIngressOutcome::Admitted, AudioIngressOutcome::Rejected);
    }

    #[test]
    fn audio_ingress_reason_display_produces_snake_case() {
        assert_eq!(AudioIngressReason::Disabled.to_string(), "disabled");
        assert_eq!(
            AudioIngressReason::ChannelNotAllowed.to_string(),
            "channel_not_allowed"
        );
        assert_eq!(AudioIngressReason::FetchFailed.to_string(), "fetch_failed");
        assert_eq!(
            AudioIngressReason::MimeRejected.to_string(),
            "mime_rejected"
        );
        assert_eq!(AudioIngressReason::Oversize.to_string(), "oversize");
        assert_eq!(AudioIngressReason::TooLong.to_string(), "too_long");
        assert_eq!(AudioIngressReason::Corrupted.to_string(), "corrupted");
        assert_eq!(
            AudioIngressReason::TranscriptionFailed.to_string(),
            "transcription_failed"
        );
        assert_eq!(
            AudioIngressReason::NoSpeechDetected.to_string(),
            "no_speech_detected"
        );
        assert_eq!(
            AudioIngressReason::TranscriberUnavailable.to_string(),
            "transcriber_unavailable"
        );
        assert_eq!(AudioIngressReason::SystemError.to_string(), "system_error");
    }

    #[test]
    fn audio_ingress_event_construction_and_field_access() {
        let event = AudioIngressEvent {
            channel: "telegram".into(),
            outcome: AudioIngressOutcome::Admitted,
            reason: None,
            mime_type: Some("audio/ogg".into()),
            byte_len: Some(50_000),
            duration_secs: Some(15.5),
            transcription_duration_ms: Some(3200),
        };
        assert_eq!(event.channel, "telegram");
        assert_eq!(event.outcome, AudioIngressOutcome::Admitted);
        assert!(event.reason.is_none());
        assert_eq!(event.mime_type.as_deref(), Some("audio/ogg"));
        assert_eq!(event.byte_len, Some(50_000));
        assert_eq!(event.duration_secs, Some(15.5));
        assert_eq!(event.transcription_duration_ms, Some(3200));
    }

    #[test]
    fn audio_ingress_event_rejected_with_reason() {
        let event = AudioIngressEvent {
            channel: "telegram".into(),
            outcome: AudioIngressOutcome::Rejected,
            reason: Some(AudioIngressReason::Oversize),
            mime_type: None,
            byte_len: Some(30_000_000),
            duration_secs: None,
            transcription_duration_ms: None,
        };
        assert_eq!(event.outcome, AudioIngressOutcome::Rejected);
        assert_eq!(event.reason, Some(AudioIngressReason::Oversize));
    }

    #[test]
    fn audio_ingress_event_is_cloneable() {
        let event = AudioIngressEvent {
            channel: "telegram".into(),
            outcome: AudioIngressOutcome::Rejected,
            reason: Some(AudioIngressReason::Disabled),
            mime_type: None,
            byte_len: None,
            duration_secs: None,
            transcription_duration_ms: None,
        };
        let cloned = event.clone();
        assert_eq!(cloned.channel, "telegram");
        assert_eq!(cloned.outcome, AudioIngressOutcome::Rejected);
    }

    #[test]
    fn observer_event_audio_ingress_variant_exists() {
        let event = ObserverEvent::AudioIngress(AudioIngressEvent {
            channel: "telegram".into(),
            outcome: AudioIngressOutcome::Rejected,
            reason: Some(AudioIngressReason::ChannelNotAllowed),
            mime_type: None,
            byte_len: None,
            duration_secs: None,
            transcription_duration_ms: None,
        });
        assert!(matches!(event, ObserverEvent::AudioIngress(_)));
    }

    #[test]
    fn observer_default_on_audio_ingress_forwards_to_record_event() {
        let observer = DummyObserver::default();
        let event = AudioIngressEvent {
            channel: "telegram".into(),
            outcome: AudioIngressOutcome::Rejected,
            reason: Some(AudioIngressReason::Disabled),
            mime_type: None,
            byte_len: None,
            duration_secs: None,
            transcription_duration_ms: None,
        };
        observer.on_audio_ingress(&event);
        assert_eq!(*observer.events.lock(), 1);
    }

    #[test]
    fn observer_default_on_image_ingress_forwards_to_record_event() {
        let observer = DummyObserver::default();
        let event = ImageIngressEvent {
            channel: "telegram".into(),
            provider: None,
            model: None,
            outcome: ImageIngressOutcome::Rejected,
            reason: Some(ImageIngressReason::Disabled),
            image_count: 1,
            max_images_per_turn: None,
            images: Vec::new(),
            total_byte_len: None,
            mime_type: None,
            byte_len: None,
        };
        observer.on_image_ingress(&event);
        // Default forwards to record_event
        assert_eq!(*observer.events.lock(), 1);
    }

    #[test]
    fn image_ingress_multi_image_turn_uses_turn_level_fields() {
        let event = ImageIngressEvent {
            channel: "telegram".into(),
            provider: Some("compatible".into()),
            model: Some("vision-model".into()),
            outcome: ImageIngressOutcome::ProviderSent,
            reason: None,
            image_count: 3,
            max_images_per_turn: None,
            images: vec![
                ImageIngressImageMeta {
                    mime_type: "image/jpeg".into(),
                    byte_len: 100,
                },
                ImageIngressImageMeta {
                    mime_type: "image/png".into(),
                    byte_len: 200,
                },
                ImageIngressImageMeta {
                    mime_type: "image/webp".into(),
                    byte_len: 300,
                },
            ],
            total_byte_len: Some(600),
            mime_type: None,
            byte_len: None,
        };

        assert_eq!(event.image_count, 3);
        assert_eq!(event.images[0].mime_type, "image/jpeg");
        assert_eq!(event.images[2].byte_len, 300);
        assert_eq!(event.total_byte_len, Some(600));
        assert!(event.mime_type.is_none());
        assert!(event.byte_len.is_none());
    }

    #[test]
    fn image_ingress_rejected_turn_can_include_effective_limit() {
        let event = ImageIngressEvent {
            channel: "telegram".into(),
            provider: None,
            model: None,
            outcome: ImageIngressOutcome::Rejected,
            reason: Some(ImageIngressReason::TooManyImages),
            image_count: 6,
            max_images_per_turn: Some(4),
            images: Vec::new(),
            total_byte_len: None,
            mime_type: None,
            byte_len: None,
        };

        assert_eq!(event.image_count, 6);
        assert_eq!(event.max_images_per_turn, Some(4));
    }
}
