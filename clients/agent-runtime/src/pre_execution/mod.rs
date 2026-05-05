use crate::agent::unified_entrypoint::{self, CanonicalOutcome};
use crate::memory::Memory;
use crate::session_commands::{
    default_registry, CommandContext, SessionCommandOutcome, SessionCommandService,
    SessionCommandToolEntry,
};

mod session_command_adapter;

pub use session_command_adapter::{
    adapt_handled_ingress, HandledIngress, HandledIngressOutcome, SessionCommandFailureClass,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockingOutcome {
    ApprovalRequired { tool: String, reason: String },
    TimeoutAborted,
    Fallback { response: String },
}

#[derive(Debug, Clone)]
pub enum IngressDecision {
    Continue,
    Blocking(BlockingOutcome),
    SessionCommand { outcome: SessionCommandOutcome },
}

pub fn approval_granted_from_env() -> bool {
    std::env::var("CORVUS_UNIFIED_APPROVE").as_deref() == Ok("1")
}

pub fn test_triggers_enabled() -> bool {
    cfg!(test) || std::env::var("CORVUS_UNIFIED_TEST_TRIGGERS").as_deref() == Ok("1")
}

pub async fn evaluate(session_id: String, prompt: &str) -> CanonicalOutcome {
    unified_entrypoint::run_canonical_outcome(
        session_id,
        prompt,
        approval_granted_from_env(),
        unified_entrypoint::CanonicalOutcomeConfig {
            enable_test_triggers: test_triggers_enabled(),
        },
    )
    .await
}

pub async fn evaluate_ingress(
    memory: &dyn Memory,
    tool_snapshot: &[SessionCommandToolEntry],
    context: CommandContext,
    prompt: &str,
    include_blocking_fallback: bool,
) -> IngressDecision {
    let service = SessionCommandService::with_tool_snapshot(memory, tool_snapshot);
    let session_id = context.session.session_id.clone();
    if let Some(outcome) = default_registry()
        .dispatch_prompt(&service, context, prompt)
        .await
    {
        return IngressDecision::SessionCommand { outcome };
    }

    if !include_blocking_fallback {
        return IngressDecision::Continue;
    }

    let canonical = evaluate(session_id.to_string(), prompt).await;
    match classify_blocking(&canonical) {
        Some(blocking) => IngressDecision::Blocking(blocking),
        None => IngressDecision::Continue,
    }
}

pub fn classify_blocking(outcome: &CanonicalOutcome) -> Option<BlockingOutcome> {
    if let Some(tool) = &outcome.approval_required {
        return Some(BlockingOutcome::ApprovalRequired {
            tool: tool.clone(),
            reason: outcome
                .approval_reason
                .clone()
                .unwrap_or_else(|| format!("approval required for `{tool}`")),
        });
    }

    if outcome.timeout_aborted {
        return Some(BlockingOutcome::TimeoutAborted);
    }

    outcome
        .fallback_response
        .as_ref()
        .map(|response| BlockingOutcome::Fallback {
            response: response.clone(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ExecutionMode;
    use crate::memory::{Memory, MemoryCategory, MemoryEntry};
    use crate::session_commands::{
        CommandCaller, CommandIngressSource, CommandSessionSource, SessionCommandFailure,
        SessionCommandFailureKind, SessionCommandOutcome, SessionCommandSuccess,
        SessionCommandSuccessData, SessionCommandToolEntry, SessionCommandToolSourceKind,
    };
    use async_trait::async_trait;

    fn assert_same_ingress_decision(left: IngressDecision, right: IngressDecision, context: &str) {
        match (left, right) {
            (IngressDecision::Continue, IngressDecision::Continue) => {}
            (IngressDecision::Blocking(left), IngressDecision::Blocking(right)) => {
                assert_eq!(left, right, "{context}");
            }
            (
                IngressDecision::SessionCommand { outcome: left },
                IngressDecision::SessionCommand { outcome: right },
            ) => {
                assert_eq!(left, right, "{context}");
            }
            (left, right) => panic!("{context}: left={left:?}, right={right:?}"),
        }
    }

    struct IngressMemory;

    #[async_trait]
    impl Memory for IngressMemory {
        fn name(&self) -> &str {
            "none"
        }

        async fn store(
            &self,
            _key: &str,
            _content: &str,
            _category: MemoryCategory,
            _session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    #[test]
    fn classify_prefers_approval_required() {
        let outcome = CanonicalOutcome {
            session_id: "s1".to_string(),
            events: Vec::new(),
            approval_required: Some("tool-1".to_string()),
            approval_reason: None,
            timeout_aborted: true,
            fallback_response: Some("fallback".to_string()),
        };

        assert_eq!(
            classify_blocking(&outcome),
            Some(BlockingOutcome::ApprovalRequired {
                tool: "tool-1".to_string(),
                reason: "approval required for `tool-1`".to_string(),
            })
        );
    }

    #[test]
    fn classify_identifies_timeout() {
        let outcome = CanonicalOutcome {
            session_id: "s1".to_string(),
            events: Vec::new(),
            approval_required: None,
            approval_reason: None,
            timeout_aborted: true,
            fallback_response: None,
        };

        assert_eq!(
            classify_blocking(&outcome),
            Some(BlockingOutcome::TimeoutAborted)
        );
    }

    #[test]
    fn classify_identifies_fallback() {
        let outcome = CanonicalOutcome {
            session_id: "s1".to_string(),
            events: Vec::new(),
            approval_required: None,
            approval_reason: None,
            timeout_aborted: false,
            fallback_response: Some("fallback response".to_string()),
        };

        assert_eq!(
            classify_blocking(&outcome),
            Some(BlockingOutcome::Fallback {
                response: "fallback response".to_string(),
            })
        );
    }

    #[test]
    fn classify_returns_none_when_nothing_blocks() {
        let outcome = CanonicalOutcome {
            session_id: "s1".to_string(),
            events: Vec::new(),
            approval_required: None,
            approval_reason: None,
            timeout_aborted: false,
            fallback_response: None,
        };

        assert_eq!(classify_blocking(&outcome), None);
    }

    #[test]
    fn classify_prefers_timeout_over_fallback_when_no_approval() {
        let outcome = CanonicalOutcome {
            session_id: "s1".to_string(),
            events: Vec::new(),
            approval_required: None,
            approval_reason: None,
            timeout_aborted: true,
            fallback_response: Some("fallback response".to_string()),
        };

        assert_eq!(
            classify_blocking(&outcome),
            Some(BlockingOutcome::TimeoutAborted)
        );
    }

    #[tokio::test]
    async fn ingress_classifies_in_scope_session_commands_through_shared_seam() {
        for prompt in ["/resume", "/suspend", "/tldr", "/compact", "/session"] {
            let decision = evaluate_ingress(
                &IngressMemory,
                &[],
                CommandContext::for_cli(
                    "session-1",
                    CommandSessionSource::Existing,
                    ExecutionMode::Standard,
                    None,
                ),
                prompt,
                true,
            )
            .await;

            match decision {
                IngressDecision::SessionCommand { outcome } => {
                    if prompt == "/session" {
                        assert!(matches!(
                            outcome,
                            SessionCommandOutcome::Success(SessionCommandSuccess {
                                command: "/session",
                                data: SessionCommandSuccessData::SessionHelp { .. },
                                ..
                            })
                        ));
                    } else if prompt == "/resume" {
                        assert_eq!(
                            outcome,
                            SessionCommandOutcome::Failure(
                                crate::session_commands::SessionCommandFailure {
                                    command: "/resume",
                                    kind: SessionCommandFailureKind::MissingCallerScope,
                                    session_id: Some("session-1".to_string()),
                                    message: "permission denied: caller scope unavailable"
                                        .to_string(),
                                }
                            )
                        );
                    } else {
                        assert_eq!(
                            outcome,
                            SessionCommandOutcome::Failure(
                                crate::session_commands::SessionCommandFailure {
                                    command: prompt,
                                    kind: SessionCommandFailureKind::UnsupportedBackend,
                                    session_id: Some("session-1".to_string()),
                                    message: "slash-session commands require sqlite memory backend (backend=none)"
                                        .to_string(),
                                }
                            )
                        );
                    }
                }
                other => {
                    panic!("expected session command interception for {prompt}, got {other:?}")
                }
            }
        }
    }

    #[tokio::test]
    async fn ingress_classifies_session_status_through_shared_seam() {
        let decision = evaluate_ingress(
            &IngressMemory,
            &[],
            CommandContext::for_cli(
                "session-1",
                CommandSessionSource::Existing,
                ExecutionMode::Standard,
                None,
            ),
            "/session status",
            true,
        )
        .await;

        match decision {
            IngressDecision::SessionCommand { outcome } => {
                assert!(matches!(
                    outcome,
                    SessionCommandOutcome::Failure(SessionCommandFailure {
                        command: "/session",
                        kind: SessionCommandFailureKind::UnsupportedBackend,
                        ..
                    })
                ));
            }
            other => panic!("expected /session status interception, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ingress_classifies_session_inspect_through_shared_seam() {
        let decision = evaluate_ingress(
            &IngressMemory,
            &[],
            CommandContext::for_cli(
                "session-1",
                CommandSessionSource::Existing,
                ExecutionMode::Standard,
                None,
            ),
            "/session inspect",
            true,
        )
        .await;

        match decision {
            IngressDecision::SessionCommand { outcome } => {
                assert!(matches!(
                    outcome,
                    SessionCommandOutcome::Failure(SessionCommandFailure {
                        command: "/session",
                        kind: SessionCommandFailureKind::UnsupportedBackend,
                        ..
                    })
                ));
            }
            other => panic!("expected /session inspect interception, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ingress_classifies_session_list_through_shared_seam() {
        let decision = evaluate_ingress(
            &IngressMemory,
            &[],
            CommandContext::for_cli(
                "session-1",
                CommandSessionSource::Existing,
                ExecutionMode::Standard,
                Some("scope-a".to_string()),
            ),
            "/session list",
            true,
        )
        .await;

        match decision {
            IngressDecision::SessionCommand { outcome } => {
                assert!(matches!(
                    outcome,
                    SessionCommandOutcome::Failure(SessionCommandFailure {
                        command: "/session",
                        kind: SessionCommandFailureKind::UnsupportedBackend,
                        ..
                    })
                ));
            }
            other => panic!("expected /session list interception, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ingress_keeps_unsupported_session_subcommands_inside_family_handler_boundary() {
        let decision = evaluate_ingress(
            &IngressMemory,
            &[],
            CommandContext::for_cli(
                "session-1",
                CommandSessionSource::Existing,
                ExecutionMode::Standard,
                None,
            ),
            "/session archive",
            true,
        )
        .await;

        match decision {
            IngressDecision::SessionCommand { outcome } => {
                assert!(matches!(
                    outcome,
                    SessionCommandOutcome::Failure(SessionCommandFailure {
                        command: "/session",
                        kind: SessionCommandFailureKind::InvalidArguments,
                        ..
                    })
                ));
            }
            other => panic!("expected /session family interception, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ingress_preserves_unknown_slash_like_input() {
        let decision = evaluate_ingress(
            &IngressMemory,
            &[],
            CommandContext::for_cli(
                "session-1",
                CommandSessionSource::Existing,
                ExecutionMode::Standard,
                None,
            ),
            "/resume-later",
            true,
        )
        .await;

        assert!(matches!(decision, IngressDecision::Continue));
    }

    #[tokio::test]
    async fn ingress_reports_invalid_argument_shape_for_recognized_command() {
        let decision = evaluate_ingress(
            &IngressMemory,
            &[],
            CommandContext::for_cli(
                "session-1",
                CommandSessionSource::Existing,
                ExecutionMode::Standard,
                None,
            ),
            "/tldr extra args",
            true,
        )
        .await;

        match decision {
            IngressDecision::SessionCommand { outcome } => {
                assert!(matches!(
                    outcome,
                    SessionCommandOutcome::Failure(
                        crate::session_commands::SessionCommandFailure {
                            kind: SessionCommandFailureKind::InvalidArguments,
                            ..
                        }
                    )
                ));
            }
            other => panic!("expected session command error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ingress_routes_tools_through_shared_seam_with_tool_snapshot() {
        let tools = vec![SessionCommandToolEntry {
            name: "shell".to_string(),
            description: "Execute shell commands".to_string(),
            source_kind: SessionCommandToolSourceKind::Native,
            source_label: None,
            aliases: vec![],
        }];

        let decision = evaluate_ingress(
            &IngressMemory,
            &tools,
            CommandContext::for_cli(
                "session-1",
                CommandSessionSource::Existing,
                ExecutionMode::Standard,
                None,
            ),
            "/tools",
            true,
        )
        .await;

        match decision {
            IngressDecision::SessionCommand { outcome } => {
                assert!(matches!(
                    outcome,
                    SessionCommandOutcome::Success(SessionCommandSuccess {
                        command: "/tools",
                        ref message,
                        data: SessionCommandSuccessData::ToolListing { .. },
                        ..
                    }) if message.contains("Available tools (1):")
                ));
            }
            other => panic!("expected /tools session command interception, got {other:?}"),
        }
    }

    #[test]
    fn session_command_regression_registry_freezes_current_session_commands() {
        let registry = crate::session_commands::default_registry();
        let descriptors = registry
            .iter()
            .map(|descriptor| {
                (
                    descriptor.canonical_name,
                    descriptor.aliases,
                    descriptor.argument_shape.clone(),
                    descriptor.requirements.capabilities,
                    descriptor.requirements.permissions,
                    descriptor.requirements.backends,
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            descriptors,
            vec![
                (
                    "/tools",
                    &[] as &[&str],
                    crate::session_commands::SlashCommandArgumentShape::None,
                    &[] as &[crate::session_commands::CommandCapability],
                    &[] as &[crate::session_commands::CommandPermission],
                    &[] as &[crate::session_commands::CommandBackend],
                ),
                (
                    "/resume",
                    &["/continue"] as &[&str],
                    crate::session_commands::SlashCommandArgumentShape::OptionalTargetThenText,
                    &[crate::session_commands::CommandCapability::SessionLifecycle]
                        as &[crate::session_commands::CommandCapability],
                    &[
                        crate::session_commands::CommandPermission::RequiresCallerScope,
                        crate::session_commands::CommandPermission::RequiresResumableSessionVisibility,
                    ] as &[crate::session_commands::CommandPermission],
                    &[crate::session_commands::CommandBackend::SqliteSlashSessions]
                        as &[crate::session_commands::CommandBackend],
                ),
                (
                    "/suspend",
                    &[] as &[&str],
                    crate::session_commands::SlashCommandArgumentShape::None,
                    &[crate::session_commands::CommandCapability::SessionLifecycle]
                        as &[crate::session_commands::CommandCapability],
                    &[] as &[crate::session_commands::CommandPermission],
                    &[crate::session_commands::CommandBackend::SqliteSlashSessions]
                        as &[crate::session_commands::CommandBackend],
                ),
                (
                    "/tldr",
                    &[] as &[&str],
                    crate::session_commands::SlashCommandArgumentShape::None,
                    &[crate::session_commands::CommandCapability::SessionSummary]
                        as &[crate::session_commands::CommandCapability],
                    &[] as &[crate::session_commands::CommandPermission],
                    &[crate::session_commands::CommandBackend::SqliteSlashSessions]
                        as &[crate::session_commands::CommandBackend],
                ),
                (
                    "/compact",
                    &[] as &[&str],
                    crate::session_commands::SlashCommandArgumentShape::OptionalText,
                    &[crate::session_commands::CommandCapability::SessionSummary]
                        as &[crate::session_commands::CommandCapability],
                    &[] as &[crate::session_commands::CommandPermission],
                    &[crate::session_commands::CommandBackend::SqliteSlashSessions]
                        as &[crate::session_commands::CommandBackend],
                ),
                (
                    "/session",
                    &[] as &[&str],
                    crate::session_commands::SlashCommandArgumentShape::OptionalText,
                    &[crate::session_commands::CommandCapability::SessionRead]
                        as &[crate::session_commands::CommandCapability],
                    &[] as &[crate::session_commands::CommandPermission],
                    &[] as &[crate::session_commands::CommandBackend],
                ),
                (
                    "/model",
                    &[] as &[&str],
                    crate::session_commands::SlashCommandArgumentShape::OptionalText,
                    &[crate::session_commands::CommandCapability::SettingsRead]
                        as &[crate::session_commands::CommandCapability],
                    &[] as &[crate::session_commands::CommandPermission],
                    &[] as &[crate::session_commands::CommandBackend],
                ),
                (
                    "/provider",
                    &[] as &[&str],
                    crate::session_commands::SlashCommandArgumentShape::OptionalText,
                    &[crate::session_commands::CommandCapability::SettingsRead]
                        as &[crate::session_commands::CommandCapability],
                    &[] as &[crate::session_commands::CommandPermission],
                    &[] as &[crate::session_commands::CommandBackend],
                ),
                (
                    "/temperature",
                    &[] as &[&str],
                    crate::session_commands::SlashCommandArgumentShape::None,
                    &[crate::session_commands::CommandCapability::SettingsRead]
                        as &[crate::session_commands::CommandCapability],
                    &[] as &[crate::session_commands::CommandPermission],
                    &[] as &[crate::session_commands::CommandBackend],
                ),
                (
                    "/mcp",
                    &[] as &[&str],
                    crate::session_commands::SlashCommandArgumentShape::RequiredText,
                    &[crate::session_commands::CommandCapability::McpManagement]
                        as &[crate::session_commands::CommandCapability],
                    &[] as &[crate::session_commands::CommandPermission],
                    &[] as &[crate::session_commands::CommandBackend],
                ),
                (
                    "/tool",
                    &[] as &[&str],
                    crate::session_commands::SlashCommandArgumentShape::RequiredText,
                    &[crate::session_commands::CommandCapability::ToolManagement]
                        as &[crate::session_commands::CommandCapability],
                    &[] as &[crate::session_commands::CommandPermission],
                    &[] as &[crate::session_commands::CommandBackend],
                ),
            ]
        );
    }

    #[tokio::test]
    async fn session_command_regression_invalid_inputs_have_normalized_failures() {
        let cases = [
            (
                "/tools extra",
                "/tools",
                SessionCommandFailureKind::InvalidArguments,
                "invalid slash command usage for /tools: this command does not accept trailing arguments",
            ),
            (
                "/mcp",
                "/mcp",
                SessionCommandFailureKind::InvalidArguments,
                "invalid slash command usage for /mcp: a subcommand argument is required",
            ),
            (
                "/tool toggle shell",
                "/tool",
                SessionCommandFailureKind::InvalidArguments,
                "Unknown /tool subcommand: 'toggle'. Use enable or disable.",
            ),
            (
                "/session archive",
                "/session",
                SessionCommandFailureKind::InvalidArguments,
                "Unknown /session subcommand: 'archive'. Usage: /session, /session status, /session inspect, or /session list",
            ),
        ];

        for (prompt, command, kind, message) in cases {
            let decision = evaluate_ingress(
                &IngressMemory,
                &[],
                CommandContext::for_cli(
                    "session-1",
                    CommandSessionSource::Existing,
                    ExecutionMode::Standard,
                    None,
                ),
                prompt,
                false,
            )
            .await;

            match decision {
                IngressDecision::SessionCommand {
                    outcome: SessionCommandOutcome::Failure(failure),
                } => {
                    assert_eq!(failure.command, command, "command drift for {prompt}");
                    assert_eq!(failure.kind, kind, "failure kind drift for {prompt}");
                    assert_eq!(failure.message, message, "message drift for {prompt}");
                    if message.starts_with("invalid slash command usage") {
                        assert_eq!(failure.session_id, None, "argument-shape failures stay pre-context");
                    } else {
                        assert_eq!(failure.session_id, Some("session-1".to_string()));
                    }
                }
                other => panic!("prompt {prompt} should fail as session command, got {other:?}"),
            }
        }

        assert!(matches!(
            evaluate_ingress(
                &IngressMemory,
                &[],
                CommandContext::for_cli(
                    "session-1",
                    CommandSessionSource::Existing,
                    ExecutionMode::Standard,
                    None,
                ),
                "/unknown",
                false,
            )
            .await,
            IngressDecision::Continue
        ));
    }

    #[tokio::test]
    async fn session_command_regression_cli_gateway_transport_parity() {
        let tools = vec![SessionCommandToolEntry {
            name: "shell".to_string(),
            description: "Execute shell commands".to_string(),
            source_kind: SessionCommandToolSourceKind::Native,
            source_label: None,
            aliases: vec![],
        }];
        let prompts = [
            "/tools",
            "/session",
            "/session list",
            "/model gpt-4o",
            "/provider openai",
            "/temperature",
            "/mcp list",
            "/mcp add local-server",
            "/tool enable shell",
            "/tool disable shell",
            "/tool toggle shell",
            "/resume",
        ];

        for prompt in prompts {
            let cli = evaluate_ingress(
                &IngressMemory,
                &tools,
                CommandContext::for_cli(
                    "session-1",
                    CommandSessionSource::Existing,
                    ExecutionMode::Standard,
                    None,
                ),
                prompt,
                false,
            )
            .await;
            let gateway = evaluate_ingress(
                &IngressMemory,
                &tools,
                CommandContext::for_gateway_http(
                    "session-1",
                    CommandSessionSource::Existing,
                    ExecutionMode::Standard,
                    None,
                ),
                prompt,
                false,
            )
            .await;

            assert_same_ingress_decision(cli, gateway, &format!("transport parity drift for {prompt}"));
        }
    }

    #[tokio::test]
    async fn session_command_regression_plan_mode_keeps_slash_command_path() {
        let prompts = ["/tools", "/session", "/model planning-model", "/mcp list"];

        for prompt in prompts {
            let standard = evaluate_ingress(
                &IngressMemory,
                &[],
                CommandContext::for_cli(
                    "session-1",
                    CommandSessionSource::Existing,
                    ExecutionMode::Standard,
                    None,
                ),
                prompt,
                false,
            )
            .await;
            let plan = evaluate_ingress(
                &IngressMemory,
                &[],
                CommandContext::for_cli(
                    "session-1",
                    CommandSessionSource::Existing,
                    ExecutionMode::Plan,
                    None,
                ),
                prompt,
                false,
            )
            .await;

            assert_same_ingress_decision(
                standard,
                plan,
                &format!("plan mode changed slash behavior for {prompt}"),
            );
        }
    }

    #[tokio::test]
    async fn session_command_regression_authz_and_ownership_enforcement_is_transport_stable() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let memory = crate::memory::SqliteMemory::new(temp_dir.path()).unwrap();
        memory
            .upsert_session("session-target", Some("owner-scope"))
            .await
            .unwrap();
        let snapshot = memory
            .create_session_snapshot(
                "session-target",
                crate::memory::SessionSnapshotKind::Compact,
                serde_json::json!({
                    "preview": "resume me",
                    "summary": "resume me",
                    "resume_context": "resume me",
                }),
                true,
            )
            .await
            .unwrap();
        memory
            .apply_session_state_patch(crate::memory::SessionStatePatch {
                session_id: "session-target".to_string(),
                lifecycle: Some(crate::memory::SlashSessionLifecycle::Suspended),
                latest_tldr_snapshot_id: crate::memory::SessionFieldPatch::Keep,
                latest_compact_snapshot_id: crate::memory::SessionFieldPatch::Set(snapshot.id),
                pending_hydration_snapshot_id: crate::memory::SessionFieldPatch::Clear,
                suspended_at: crate::memory::SessionFieldPatch::Set("now".to_string()),
            })
            .await
            .unwrap();

        let missing_scope = evaluate_ingress(
            &memory,
            &[],
            CommandContext::for_gateway_http(
                "session-control",
                CommandSessionSource::Explicit,
                ExecutionMode::Standard,
                None,
            ),
            "/resume session-target",
            false,
        )
        .await;
        assert!(matches!(
            missing_scope,
            IngressDecision::SessionCommand {
                outcome: SessionCommandOutcome::Failure(SessionCommandFailure {
                    command: "/resume",
                    kind: SessionCommandFailureKind::MissingCallerScope,
                    ..
                })
            }
        ));

        let cli_wrong_owner = evaluate_ingress(
            &memory,
            &[],
            CommandContext::for_cli(
                "session-control",
                CommandSessionSource::Explicit,
                ExecutionMode::Standard,
                Some("other-scope".to_string()),
            ),
            "/resume session-target",
            false,
        )
        .await;
        let gateway_wrong_owner = evaluate_ingress(
            &memory,
            &[],
            CommandContext::for_gateway_http(
                "session-control",
                CommandSessionSource::Explicit,
                ExecutionMode::Standard,
                Some("other-scope".to_string()),
            ),
            "/resume session-target",
            false,
        )
        .await;

        assert_same_ingress_decision(
            cli_wrong_owner.clone(),
            gateway_wrong_owner,
            "ownership failure differs by transport",
        );
        assert!(matches!(
            cli_wrong_owner,
            IngressDecision::SessionCommand {
                outcome: SessionCommandOutcome::Failure(SessionCommandFailure {
                    command: "/resume",
                    kind: SessionCommandFailureKind::PermissionDenied,
                    ..
                })
            }
        ));
    }

    #[test]
    fn adapt_handled_ingress_returns_not_handled_for_continue() {
        assert!(matches!(
            adapt_handled_ingress(IngressDecision::Continue),
            HandledIngress::NotHandled
        ));
    }

    #[test]
    fn adapt_handled_ingress_preserves_success_outcome() {
        let success = SessionCommandSuccess {
            command: "/tldr",
            session_id: "session-1".to_string(),
            message: "summary".to_string(),
            data: SessionCommandSuccessData::None,
        };

        assert!(matches!(
            adapt_handled_ingress(IngressDecision::SessionCommand {
                outcome: SessionCommandOutcome::Success(success.clone()),
            }),
            HandledIngress::Handled(HandledIngressOutcome::SessionCommandSuccess(actual))
                if actual.as_ref() == &success
        ));
    }

    #[test]
    fn adapt_handled_ingress_classifies_permission_failures() {
        for kind in [
            SessionCommandFailureKind::MissingCallerScope,
            SessionCommandFailureKind::PermissionDenied,
        ] {
            let failure = SessionCommandFailure {
                command: "/resume",
                kind: kind.clone(),
                session_id: Some("session-1".to_string()),
                message: "denied".to_string(),
            };

            assert!(matches!(
                adapt_handled_ingress(IngressDecision::SessionCommand {
                    outcome: SessionCommandOutcome::Failure(failure.clone()),
                }),
                HandledIngress::Handled(HandledIngressOutcome::SessionCommandFailure {
                    class: SessionCommandFailureClass::PermissionDenied,
                    failure: actual,
                }) if actual == failure
            ));
        }
    }

    #[test]
    fn adapt_handled_ingress_classifies_generic_failures() {
        let failure = SessionCommandFailure {
            command: "/tldr",
            kind: SessionCommandFailureKind::InvalidArguments,
            session_id: Some("session-1".to_string()),
            message: "bad args".to_string(),
        };

        assert!(matches!(
            adapt_handled_ingress(IngressDecision::SessionCommand {
                outcome: SessionCommandOutcome::Failure(failure.clone()),
            }),
            HandledIngress::Handled(HandledIngressOutcome::SessionCommandFailure {
                class: SessionCommandFailureClass::Failed,
                failure: actual,
            }) if actual == failure
        ));
    }

    #[test]
    fn adapt_handled_ingress_preserves_blocking_outcomes() {
        let blocking = BlockingOutcome::Fallback {
            response: "fallback response".to_string(),
        };

        assert!(matches!(
            adapt_handled_ingress(IngressDecision::Blocking(blocking.clone())),
            HandledIngress::Handled(HandledIngressOutcome::Blocking(actual)) if actual == blocking
        ));
    }

    #[test]
    fn typed_context_builders_preserve_transport_specific_caller_semantics() {
        let cli = CommandContext::for_cli(
            "session-cli",
            CommandSessionSource::Existing,
            ExecutionMode::Standard,
            Some("cli-scope".to_string()),
        );
        let gateway = CommandContext::for_gateway_http(
            "session-http",
            CommandSessionSource::Explicit,
            ExecutionMode::Plan,
            Some("verified-scope".to_string()),
        );
        let channel = CommandContext::for_channel(
            "session-channel",
            CommandSessionSource::Generated,
            ExecutionMode::Standard,
            "discord",
            Some("channel-scope".to_string()),
        );

        assert!(matches!(cli.ingress.source, CommandIngressSource::Cli));
        assert!(matches!(
            cli.caller,
            CommandCaller::DerivedCliScope { ref scope_key } if scope_key == "cli-scope"
        ));

        assert!(matches!(
            gateway.ingress.source,
            CommandIngressSource::GatewayHttp
        ));
        assert!(matches!(
            gateway.ingress.execution_mode,
            ExecutionMode::Plan
        ));
        assert!(matches!(
            gateway.caller,
            CommandCaller::VerifiedTokenHash { ref scope_key } if scope_key == "verified-scope"
        ));

        assert!(matches!(
            channel.ingress.source,
            CommandIngressSource::Channel { ref name } if name == "discord"
        ));
        assert!(matches!(
            channel.caller,
            CommandCaller::DerivedChannelScope {
                ref channel,
                ref scope_key,
            } if channel == "discord" && scope_key == "channel-scope"
        ));
    }
}
