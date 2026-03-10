use super::traits::{redact_observer_payload, Observer, ObserverEvent, ObserverMetric};
use std::any::Any;
use tracing::info;

/// Log-based observer — uses tracing, zero external deps
pub struct LogObserver;

impl LogObserver {
    pub fn new() -> Self {
        Self
    }
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
        }
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
}
