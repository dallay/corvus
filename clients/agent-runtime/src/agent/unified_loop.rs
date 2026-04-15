use std::sync::Mutex;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct LoopConfig {
    pub max_iterations: usize,
    pub timeout: Duration,
    pub compaction_threshold: usize,
    pub approval_required_tool: Option<String>,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            timeout: Duration::from_secs(60),
            compaction_threshold: 4_096,
            approval_required_tool: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoopEvent {
    Start,
    LLMProgress(String),
    ToolDispatchStarted(String),
    ToolDispatchCompleted(String),
    CompactionTriggered,
    ApprovalRequired(String, String),
    Complete(String),
    Error(String),
}

pub struct AgentLoop {
    pub config: LoopConfig,
    pending_approval: Mutex<Option<String>>,
}

impl AgentLoop {
    fn prompt_progress_summary(prompt: &str) -> String {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        prompt.hash(&mut hasher);
        let fingerprint = hasher.finish();
        format!(
            "prompt_received chars={} hash={fingerprint:016x}",
            prompt.chars().count()
        )
    }

    fn requires_approval_for(&self, tool_name: &str) -> bool {
        self.config.approval_required_tool.as_deref() == Some(tool_name)
    }

    pub fn new(config: LoopConfig) -> Self {
        Self {
            config,
            pending_approval: Mutex::new(None),
        }
    }

    pub fn run(
        &self,
        prompt: &str,
        tool_calls: usize,
        step_duration: Duration,
    ) -> impl futures::Stream<Item = LoopEvent> {
        if let Ok(mut pending) = self.pending_approval.lock() {
            *pending = None;
        }

        let mut events = vec![
            LoopEvent::Start,
            LoopEvent::LLMProgress(Self::prompt_progress_summary(prompt)),
        ];
        let mut elapsed = Duration::ZERO;
        let mut context_size = prompt.len();

        if context_size > self.config.compaction_threshold {
            events.push(LoopEvent::CompactionTriggered);
            context_size = self.config.compaction_threshold / 2;
        }

        for idx in 0..tool_calls {
            if idx >= self.config.max_iterations {
                events.push(LoopEvent::Error("iteration budget exceeded".to_string()));
                return futures::stream::iter(events);
            }

            elapsed += step_duration;
            if elapsed > self.config.timeout {
                events.push(LoopEvent::Error("timeout exceeded".to_string()));
                return futures::stream::iter(events);
            }

            let tool_name = format!("tool-{}", idx + 1);

            if idx == 0 && self.requires_approval_for(&tool_name) {
                if let Ok(mut pending) = self.pending_approval.lock() {
                    *pending = Some(tool_name.clone());
                }
                events.push(LoopEvent::ApprovalRequired(
                    tool_name.clone(),
                    format!("approval required for `{tool_name}`"),
                ));
                return futures::stream::iter(events);
            }

            events.push(LoopEvent::ToolDispatchStarted(tool_name.clone()));
            events.push(LoopEvent::ToolDispatchCompleted(tool_name));

            context_size += 64;
            if context_size > self.config.compaction_threshold {
                events.push(LoopEvent::CompactionTriggered);
                context_size = self.config.compaction_threshold / 2;
            }
        }

        events.push(LoopEvent::Complete("done".to_string()));
        futures::stream::iter(events)
    }

    pub fn resume(&self, approved: bool) -> impl futures::Stream<Item = LoopEvent> {
        let pending_tool = match self.pending_approval.lock() {
            Ok(mut lock) => lock.take(),
            Err(_) => None,
        };

        let events = match pending_tool {
            Some(tool_name) if approved => vec![
                LoopEvent::ToolDispatchStarted(tool_name.clone()),
                LoopEvent::ToolDispatchCompleted(tool_name),
                LoopEvent::Complete("done".to_string()),
            ],
            Some(_) => vec![LoopEvent::Error("approval denied".to_string())],
            None => vec![LoopEvent::Error("no pending approval".to_string())],
        };

        futures::stream::iter(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[test]
    fn test_loop_config_default() {
        let config = LoopConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.compaction_threshold, 4_096);
    }

    #[test]
    fn test_agent_loop_init() {
        let config = LoopConfig::default();
        let agent_loop = AgentLoop::new(config.clone());
        assert_eq!(agent_loop.config, config);
    }

    #[tokio::test]
    async fn test_agent_loop_run_happy_path_stream() {
        let agent_loop = AgentLoop::new(LoopConfig::default());
        let events = agent_loop
            .run("hello", 1, Duration::from_millis(5))
            .collect::<Vec<_>>()
            .await;

        assert_eq!(
            events,
            vec![
                LoopEvent::Start,
                LoopEvent::LLMProgress(AgentLoop::prompt_progress_summary("hello")),
                LoopEvent::ToolDispatchStarted("tool-1".to_string()),
                LoopEvent::ToolDispatchCompleted("tool-1".to_string()),
                LoopEvent::Complete("done".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn test_agent_loop_triggers_compaction_when_threshold_exceeded() {
        let config = LoopConfig {
            compaction_threshold: 8,
            ..LoopConfig::default()
        };
        let agent_loop = AgentLoop::new(config);
        let events = agent_loop
            .run("prompt-larger-than-threshold", 1, Duration::from_millis(1))
            .collect::<Vec<_>>()
            .await;

        assert!(events.contains(&LoopEvent::CompactionTriggered));
    }

    #[tokio::test]
    async fn test_agent_loop_emits_iteration_budget_error() {
        let config = LoopConfig {
            max_iterations: 1,
            ..LoopConfig::default()
        };
        let agent_loop = AgentLoop::new(config);
        let events = agent_loop
            .run("hi", 2, Duration::from_millis(1))
            .collect::<Vec<_>>()
            .await;

        assert!(events.iter().any(|event| {
            matches!(event, LoopEvent::Error(message) if message.contains("iteration budget exceeded"))
        }));
    }

    #[tokio::test]
    async fn test_agent_loop_emits_timeout_error() {
        let config = LoopConfig {
            timeout: Duration::from_millis(2),
            ..LoopConfig::default()
        };
        let agent_loop = AgentLoop::new(config);
        let events = agent_loop
            .run("hi", 2, Duration::from_millis(2))
            .collect::<Vec<_>>()
            .await;

        assert!(events.iter().any(|event| {
            matches!(event, LoopEvent::Error(message) if message.contains("timeout exceeded"))
        }));
    }

    #[tokio::test]
    async fn test_agent_loop_resume_continues_after_approval() {
        let agent_loop = AgentLoop::new(LoopConfig {
            approval_required_tool: Some("tool-1".to_string()),
            ..LoopConfig::default()
        });
        let run_events = agent_loop
            .run("needs-approval", 1, Duration::from_millis(1))
            .collect::<Vec<_>>()
            .await;

        assert!(run_events
            .iter()
            .any(|event| matches!(event, LoopEvent::ApprovalRequired(..))));

        let resumed = agent_loop.resume(true).collect::<Vec<_>>().await;
        assert_eq!(
            resumed,
            vec![
                LoopEvent::ToolDispatchStarted("tool-1".to_string()),
                LoopEvent::ToolDispatchCompleted("tool-1".to_string()),
                LoopEvent::Complete("done".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn test_agent_loop_resume_emits_error_when_denied() {
        let agent_loop = AgentLoop::new(LoopConfig {
            approval_required_tool: Some("tool-1".to_string()),
            ..LoopConfig::default()
        });
        let _ = agent_loop
            .run("needs-approval", 1, Duration::from_millis(1))
            .collect::<Vec<_>>()
            .await;

        let resumed = agent_loop.resume(false).collect::<Vec<_>>().await;
        assert!(resumed.iter().any(
            |event| matches!(event, LoopEvent::Error(message) if message.contains("approval denied"))
        ));
    }

    #[tokio::test]
    async fn spec_scenario_matrix_covers_contract_requirements() {
        let loop_runner = AgentLoop::new(LoopConfig::default());

        let standard = loop_runner
            .run("normal", 1, Duration::from_millis(1))
            .collect::<Vec<_>>()
            .await;
        assert!(standard
            .iter()
            .any(|event| matches!(event, LoopEvent::Start)));
        assert!(standard
            .iter()
            .any(|event| matches!(event, LoopEvent::LLMProgress(_))));
        assert!(standard
            .iter()
            .any(|event| matches!(event, LoopEvent::ToolDispatchStarted(_))));
        assert!(standard
            .iter()
            .any(|event| matches!(event, LoopEvent::ToolDispatchCompleted(_))));
        assert!(standard
            .iter()
            .any(|event| matches!(event, LoopEvent::Complete(_))));

        let compacting = AgentLoop::new(LoopConfig {
            compaction_threshold: 8,
            ..LoopConfig::default()
        })
        .run("long-context-prompt", 1, Duration::from_millis(1))
        .collect::<Vec<_>>()
        .await;
        assert!(compacting
            .iter()
            .any(|event| matches!(event, LoopEvent::CompactionTriggered)));

        let timed_out = AgentLoop::new(LoopConfig {
            timeout: Duration::from_millis(1),
            ..LoopConfig::default()
        })
        .run("timeout", 2, Duration::from_millis(2))
        .collect::<Vec<_>>()
        .await;
        assert!(timed_out.iter().any(
            |event| matches!(event, LoopEvent::Error(message) if message.contains("timeout"))
        ));

        let approval_required = AgentLoop::new(LoopConfig {
            approval_required_tool: Some("tool-1".to_string()),
            ..LoopConfig::default()
        });
        let approval_run = approval_required
            .run("needs-approval", 1, Duration::from_millis(1))
            .collect::<Vec<_>>()
            .await;
        assert!(approval_run
            .iter()
            .any(|event| matches!(event, LoopEvent::ApprovalRequired(..))));
        let approved_resume = approval_required.resume(true).collect::<Vec<_>>().await;
        assert!(approved_resume
            .iter()
            .any(|event| matches!(event, LoopEvent::Complete(_))));
    }
}
