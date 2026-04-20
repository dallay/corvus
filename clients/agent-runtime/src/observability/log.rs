use super::traits::{
    budget_state_label, cost_override_scope_label, redact_observer_payload,
    redact_optional_observer_payload, usage_period_label, BudgetOverrideEvent, Observer,
    ObserverEvent, ObserverMetric,
};
use std::any::Any;
use tracing::info;

/// Log-based observer — uses tracing, zero external deps
pub struct LogObserver;

impl LogObserver {
    pub fn new() -> Self {
        Self
    }
}

fn format_budget_override_log_payload(event: &BudgetOverrideEvent) -> String {
    format!(
        "action={} scope={} actor={} reason={} previous_state={} period={} session_id={} override_id={} surface={}",
        event.action.as_str(),
        cost_override_scope_label(event.scope),
        event.redacted_actor(),
        event
            .redacted_reason()
            .unwrap_or_else(|| "none".to_string()),
        budget_state_label(event.previous_state),
        event
            .period
            .map(usage_period_label)
            .unwrap_or("none"),
        event.session_id.as_deref().unwrap_or("none"),
        event.override_id.as_deref().unwrap_or("none"),
        redact_optional_observer_payload(event.surface.as_deref())
            .unwrap_or_else(|| "none".to_string()),
    )
}

impl Observer for LogObserver {
    fn record_event(&self, event: &ObserverEvent) {
        match event {
            ObserverEvent::AgentStart { provider, model } => {
                info!(provider = %provider, model = %model, "agent.start");
            }
            ObserverEvent::AgentEnd {
                provider,
                model,
                duration,
                tokens_used,
                cost_usd,
            } => {
                let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                info!(provider = %provider, model = %model, duration_ms = ms, tokens = ?tokens_used, cost_usd = ?cost_usd, "agent.end");
            }
            ObserverEvent::ToolCallStart { tool } => {
                info!(tool = %tool, "tool.start");
            }
            ObserverEvent::ToolCall {
                tool,
                duration,
                success,
            } => {
                let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                info!(tool = %tool, duration_ms = ms, success = success, "tool.call");
            }
            ObserverEvent::TurnComplete => {
                info!("turn.complete");
            }
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
                info!(
                    session_id = %session_id,
                    status = %status,
                    summary = %redact_observer_payload(summary),
                    files = changed_files.len(),
                    commands = commands.len(),
                    validations = validations.len(),
                    blockers = blockers.len(),
                    pending_work = pending_work.len(),
                    delegated = delegated,
                    "code_session.completed"
                );
            }
            ObserverEvent::ChannelMessage { channel, direction } => {
                info!(channel = %channel, direction = %direction, "channel.message");
            }
            ObserverEvent::HeartbeatTick => {
                info!("heartbeat.tick");
            }
            ObserverEvent::Error { component, message } => {
                info!(component = %component, error = %message, "error");
            }
            ObserverEvent::BudgetWarning(event) => {
                info!(
                    period = usage_period_label(event.period),
                    current_usd = event.current_usd,
                    projected_usd = event.projected_usd,
                    limit_usd = event.limit_usd,
                    percent_used = event.percent_used,
                    session_id = %event.session_id,
                    surface = ?event.surface,
                    "budget.warning"
                );
            }
            ObserverEvent::BudgetExceeded(event) => {
                info!(
                    period = usage_period_label(event.period),
                    current_usd = event.current_usd,
                    projected_usd = event.projected_usd,
                    limit_usd = event.limit_usd,
                    percent_used = event.percent_used,
                    session_id = %event.session_id,
                    surface = ?event.surface,
                    "budget.exceeded"
                );
            }
            ObserverEvent::BudgetOverride(event) => {
                info!(payload = %format_budget_override_log_payload(event), "budget.override");
            }
            ObserverEvent::LlmRequest {
                provider,
                model,
                messages_count,
            } => {
                info!(
                    provider = %provider,
                    model = %model,
                    messages_count = messages_count,
                    "llm.request"
                );
            }
            ObserverEvent::LlmResponse {
                provider,
                model,
                duration,
                success,
                error_message,
            } => {
                let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                info!(
                    provider = %provider,
                    model = %model,
                    duration_ms = ms,
                    success = success,
                    error = ?error_message,
                    "llm.response"
                );
            }
            ObserverEvent::MissionStarted {
                mission_id,
                checkpoint_count,
                resume_from,
            } => {
                info!(
                    mission_id = %mission_id,
                    checkpoint_count = checkpoint_count,
                    resume_from = ?resume_from,
                    "mission.started"
                );
            }
            ObserverEvent::MissionCheckpointProgress {
                mission_id,
                checkpoint_index,
                status,
                duration,
            } => {
                let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                info!(
                    mission_id = %mission_id,
                    checkpoint_index = checkpoint_index,
                    status = %status,
                    duration_ms = ms,
                    "mission.checkpoint"
                );
            }
            ObserverEvent::MissionGuardrailViolation {
                mission_id,
                checkpoint_index,
                guardrail,
                termination_reason,
                detail,
            } => {
                info!(
                    mission_id = %mission_id,
                    checkpoint_index = ?checkpoint_index,
                    guardrail = %guardrail,
                    termination_reason = %termination_reason,
                    detail = %detail,
                    "mission.guardrail_violation"
                );
            }
            ObserverEvent::MissionCompleted {
                mission_id,
                checkpoints_completed,
                duration,
            } => {
                let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                info!(
                    mission_id = %mission_id,
                    checkpoints_completed = checkpoints_completed,
                    duration_ms = ms,
                    "mission.completed"
                );
            }
            ObserverEvent::MissionTerminated {
                mission_id,
                checkpoint_index,
                termination_reason,
                duration,
                rollback,
            } => {
                let ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                info!(
                    mission_id = %mission_id,
                    checkpoint_index = ?checkpoint_index,
                    termination_reason = %termination_reason,
                    duration_ms = ms,
                    rollback = rollback,
                    "mission.terminated"
                );
            }
            ObserverEvent::ImageIngress(event) => {
                info!(
                    channel = %event.channel,
                    provider = ?event.provider,
                    model = ?event.model,
                    outcome = ?event.outcome,
                    reason = ?event.reason,
                    image_count = event.image_count,
                    max_images_per_turn = ?event.max_images_per_turn,
                    images = ?event.images,
                    total_byte_len = ?event.total_byte_len,
                    mime_type = ?event.mime_type,
                    byte_len = ?event.byte_len,
                    "image.ingress"
                );
            }
            ObserverEvent::AudioIngress(event) => {
                info!(
                    channel = %event.channel,
                    outcome = ?event.outcome,
                    reason = ?event.reason,
                    mime_type = ?event.mime_type,
                    byte_len = ?event.byte_len,
                    duration_secs = ?event.duration_secs,
                    transcription_ms = ?event.transcription_duration_ms,
                    "audio.ingress"
                );
            }
        }
    }

    fn on_image_ingress(&self, event: &super::traits::ImageIngressEvent) {
        self.record_event(&ObserverEvent::ImageIngress(event.clone()));
    }

    fn record_metric(&self, metric: &ObserverMetric) {
        match metric {
            ObserverMetric::RequestLatency(d) => {
                let ms = u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
                info!(latency_ms = ms, "metric.request_latency");
            }
            ObserverMetric::TokensUsed(t) => {
                info!(tokens = t, "metric.tokens_used");
            }
            ObserverMetric::ActiveSessions(s) => {
                info!(sessions = s, "metric.active_sessions");
            }
            ObserverMetric::QueueDepth(d) => {
                info!(depth = d, "metric.queue_depth");
            }
        }
    }

    fn name(&self) -> &str {
        "log"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::{BudgetState, CostOverrideScope, UsagePeriod};
    use crate::observability::BudgetOverrideAction;
    use std::time::Duration;

    #[test]
    fn log_observer_name() {
        assert_eq!(LogObserver::new().name(), "log");
    }

    #[test]
    fn log_observer_all_events_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::AgentStart {
            provider: "openrouter".into(),
            model: "claude-sonnet".into(),
        });
        obs.record_event(&ObserverEvent::AgentEnd {
            provider: "openrouter".into(),
            model: "claude-sonnet".into(),
            duration: Duration::from_millis(500),
            tokens_used: Some(100),
            cost_usd: Some(0.0015),
        });
        obs.record_event(&ObserverEvent::AgentEnd {
            provider: "openrouter".into(),
            model: "claude-sonnet".into(),
            duration: Duration::ZERO,
            tokens_used: None,
            cost_usd: None,
        });
        obs.record_event(&ObserverEvent::ToolCall {
            tool: "shell".into(),
            duration: Duration::from_millis(10),
            success: false,
        });
        obs.record_event(&ObserverEvent::ChannelMessage {
            channel: "telegram".into(),
            direction: "outbound".into(),
        });
        obs.record_event(&ObserverEvent::HeartbeatTick);
        obs.record_event(&ObserverEvent::Error {
            component: "provider".into(),
            message: "timeout".into(),
        });
        obs.record_event(&ObserverEvent::CodeSessionCompleted {
            session_id: "sess-1".into(),
            status: "completed".into(),
            summary: "Updated tests".into(),
            changed_files: vec!["src/lib.rs".into()],
            commands: vec!["cargo test".into()],
            validations: vec!["pass:cargo test".into()],
            blockers: vec![],
            pending_work: vec![],
            delegated: false,
        });
    }

    #[test]
    fn log_observer_all_metrics_no_panic() {
        let obs = LogObserver::new();
        obs.record_metric(&ObserverMetric::RequestLatency(Duration::from_secs(2)));
        obs.record_metric(&ObserverMetric::TokensUsed(0));
        obs.record_metric(&ObserverMetric::TokensUsed(u64::MAX));
        obs.record_metric(&ObserverMetric::ActiveSessions(1));
        obs.record_metric(&ObserverMetric::QueueDepth(999));
    }

    #[test]
    fn log_observer_tool_call_start_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::ToolCallStart {
            tool: "browser".into(),
        });
    }

    #[test]
    fn log_observer_turn_complete_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::TurnComplete);
    }

    #[test]
    fn log_observer_llm_request_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::LlmRequest {
            provider: "anthropic".into(),
            model: "claude-sonnet".into(),
            messages_count: 5,
        });
    }

    #[test]
    fn log_observer_llm_response_success_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::LlmResponse {
            provider: "openai".into(),
            model: "gpt-4o".into(),
            duration: Duration::from_millis(1200),
            success: true,
            error_message: None,
        });
    }

    #[test]
    fn log_observer_llm_response_failure_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::LlmResponse {
            provider: "openai".into(),
            model: "gpt-4o".into(),
            duration: Duration::from_millis(500),
            success: false,
            error_message: Some("rate limited".into()),
        });
    }

    #[test]
    fn log_observer_mission_started_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::MissionStarted {
            mission_id: "m-001".into(),
            checkpoint_count: 3,
            resume_from: None,
        });
    }

    #[test]
    fn log_observer_mission_started_with_resume_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::MissionStarted {
            mission_id: "m-001".into(),
            checkpoint_count: 5,
            resume_from: Some(2),
        });
    }

    #[test]
    fn log_observer_mission_checkpoint_progress_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::MissionCheckpointProgress {
            mission_id: "m-001".into(),
            checkpoint_index: 1,
            status: "running".into(),
            duration: Duration::from_secs(10),
        });
    }

    #[test]
    fn log_observer_mission_guardrail_violation_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::MissionGuardrailViolation {
            mission_id: "m-001".into(),
            checkpoint_index: Some(2),
            guardrail: "cost_limit".into(),
            termination_reason: "exceeded budget".into(),
            detail: "spent $5.00 of $3.00 limit".into(),
        });
    }

    #[test]
    fn log_observer_mission_guardrail_violation_no_checkpoint_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::MissionGuardrailViolation {
            mission_id: "m-001".into(),
            checkpoint_index: None,
            guardrail: "time_limit".into(),
            termination_reason: "timeout".into(),
            detail: "exceeded 1h limit".into(),
        });
    }

    #[test]
    fn log_observer_mission_completed_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::MissionCompleted {
            mission_id: "m-001".into(),
            checkpoints_completed: 3,
            duration: Duration::from_secs(120), // 2 minutes
        });
    }

    #[test]
    fn log_observer_mission_terminated_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::MissionTerminated {
            mission_id: "m-001".into(),
            checkpoint_index: Some(1),
            termination_reason: "guardrail".into(),
            duration: Duration::from_secs(45),
            rollback: true,
        });
    }

    #[test]
    fn log_observer_mission_terminated_no_checkpoint_no_panic() {
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::MissionTerminated {
            mission_id: "m-001".into(),
            checkpoint_index: None,
            termination_reason: "user_cancel".into(),
            duration: Duration::from_secs(5),
            rollback: false,
        });
    }

    #[test]
    fn log_observer_image_ingress_no_panic() {
        use super::super::traits::{ImageIngressEvent, ImageIngressOutcome};
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::ImageIngress(ImageIngressEvent {
            channel: "telegram".into(),
            provider: Some("gemini".into()),
            model: Some("gemini-2.0-flash".into()),
            outcome: ImageIngressOutcome::Admitted,
            reason: None,
            image_count: 2,
            max_images_per_turn: Some(4),
            images: vec![
                super::super::traits::ImageIngressImageMeta {
                    mime_type: "image/png".into(),
                    byte_len: 102_400,
                },
                super::super::traits::ImageIngressImageMeta {
                    mime_type: "image/jpeg".into(),
                    byte_len: 204_800,
                },
            ],
            total_byte_len: Some(307_200),
            mime_type: None,
            byte_len: None,
        }));
    }

    #[test]
    fn log_observer_image_ingress_rejected_no_panic() {
        use super::super::traits::{ImageIngressEvent, ImageIngressOutcome, ImageIngressReason};
        let obs = LogObserver::new();
        obs.record_event(&ObserverEvent::ImageIngress(ImageIngressEvent {
            channel: "discord".into(),
            provider: None,
            model: None,
            outcome: ImageIngressOutcome::Rejected,
            reason: Some(ImageIngressReason::MimeRejected),
            image_count: 3,
            max_images_per_turn: Some(4),
            images: Vec::new(),
            total_byte_len: None,
            mime_type: None,
            byte_len: None,
        }));
    }

    #[test]
    fn log_observer_on_image_ingress_delegates_to_record_event() {
        use super::super::traits::{ImageIngressEvent, ImageIngressOutcome};
        let obs = LogObserver::new();
        let event = ImageIngressEvent {
            channel: "slack".into(),
            provider: None,
            model: None,
            outcome: ImageIngressOutcome::ProviderSent,
            reason: None,
            image_count: 1,
            max_images_per_turn: None,
            images: vec![super::super::traits::ImageIngressImageMeta {
                mime_type: "image/jpeg".into(),
                byte_len: 128,
            }],
            total_byte_len: Some(128),
            mime_type: None,
            byte_len: None,
        };
        // Should not panic — exercises on_image_ingress path
        obs.on_image_ingress(&event);
    }

    #[test]
    fn log_observer_as_any_downcasts() {
        let obs = LogObserver::new();
        assert!(obs.as_any().downcast_ref::<LogObserver>().is_some());
    }

    #[test]
    fn log_observer_extreme_duration_no_panic() {
        let obs = LogObserver::new();
        // Duration that overflows u64 millis conversion
        obs.record_event(&ObserverEvent::AgentEnd {
            provider: "test".into(),
            model: "test".into(),
            duration: Duration::MAX,
            tokens_used: Some(u64::MAX),
            cost_usd: Some(f64::MAX),
        });
        obs.record_metric(&ObserverMetric::RequestLatency(Duration::MAX));
    }

    #[test]
    fn budget_override_log_payload_redacts_sensitive_fields() {
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

        let payload = format_budget_override_log_payload(&event);
        assert!(payload.contains("***REDACTED***"));
        assert!(!payload.contains("paired-admin-token"));
        assert!(!payload.contains("super-secret"));
    }
}
