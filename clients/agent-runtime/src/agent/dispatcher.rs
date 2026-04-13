use crate::providers::{ChatMessage, ChatResponse, ConversationMessage, ToolResultMessage};
use crate::security::{source_kind_for_tool, ExecutionOrigin, SecurityPolicy, ToolPolicyDecision};
use crate::tools::{Tool, ToolSpec};
use serde_json::Value;
use std::fmt::Write;

const SAFE_TOOL_NAMES: &[&str] = &[
    "file_read",
    "memory_recall",
    "echo",
    "mock_price",
    "counter",
];

#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    pub name: String,
    pub arguments: Value,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DispatchAction {
    Execute,
    ApprovalRequired(String), // e.g. tool name or reason
    Blocked { code: String, reason: String },
}

#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    pub name: String,
    pub output: String,
    pub success: bool,
    pub tool_call_id: Option<String>,
    pub action: DispatchAction,
}

pub trait ToolDispatcher: Send + Sync {
    fn parse_response(&self, response: &ChatResponse) -> (String, Vec<ParsedToolCall>);
    fn format_results(&self, results: &[ToolExecutionResult]) -> ConversationMessage;
    fn prompt_instructions(&self, tools: &[Box<dyn Tool>]) -> String;
    fn to_provider_messages(&self, history: &[ConversationMessage]) -> Vec<ChatMessage>;
    fn should_send_tool_specs(&self) -> bool;

    // Check if the tool invocation requires approval
    // FAIL-CLOSED: Unknown tools require approval by default for safety
    // Only explicitly safe tools can execute without approval
    fn check_tool_risk(&self, tool_name: &str, arguments: &Value) -> DispatchAction {
        self.check_tool_risk_for_origin(tool_name, arguments, ExecutionOrigin::Standard)
    }

    fn check_tool_risk_for_origin(
        &self,
        tool_name: &str,
        _arguments: &Value,
        origin: ExecutionOrigin,
    ) -> DispatchAction {
        evaluate_tool_risk_for_origin(tool_name, origin)
    }
}

pub fn evaluate_tool_risk(tool_name: &str) -> DispatchAction {
    evaluate_tool_risk_for_origin(tool_name, ExecutionOrigin::Standard)
}

pub fn evaluate_tool_risk_with_policy(tool_name: &str, policy: &SecurityPolicy) -> DispatchAction {
    evaluate_tool_risk_with_policy_for_origin(tool_name, policy, ExecutionOrigin::Standard)
}

pub fn evaluate_tool_risk_with_policy_for_origin(
    tool_name: &str,
    policy: &SecurityPolicy,
    origin: ExecutionOrigin,
) -> DispatchAction {
    // Always consult the security policy first.
    let outcome = policy.evaluate_tool_policy_outcome_for_origin(tool_name, origin);
    if outcome.decision == ToolPolicyDecision::Deny {
        return DispatchAction::Blocked {
            code: outcome.code.unwrap_or("policy_denied").to_string(),
            reason: outcome
                .reason
                .unwrap_or_else(|| format!("tool `{tool_name}` blocked by security policy")),
        };
    }

    // In Plan mode, map Allow/ApprovalRequired as currently done.
    if policy.execution_mode == crate::config::ExecutionMode::Plan {
        return match outcome.decision {
            ToolPolicyDecision::Allow => DispatchAction::Execute,
            ToolPolicyDecision::ApprovalRequired => {
                DispatchAction::ApprovalRequired(tool_name.to_string())
            }
            ToolPolicyDecision::Deny => unreachable!("deny already handled above"),
        };
    }

    // For non-Plan modes, run legacy risk checks when policy did not deny.
    evaluate_tool_risk_for_origin(tool_name, origin)
}

pub fn evaluate_tool_risk_for_origin(tool_name: &str, _origin: ExecutionOrigin) -> DispatchAction {
    match source_kind_for_tool(tool_name) {
        crate::security::ToolSourceKind::Mcp
        | crate::security::ToolSourceKind::McpResource
        | crate::security::ToolSourceKind::McpPrompt => {
            return DispatchAction::ApprovalRequired(format!(
                "mcp tool '{tool_name}' requires explicit approval"
            ));
        }
        crate::security::ToolSourceKind::Unknown | crate::security::ToolSourceKind::Native => {}
    }

    match tool_name {
        "shell" | "bash" | "execute_command" => {
            DispatchAction::ApprovalRequired(tool_name.to_string())
        }
        _ => {
            if SAFE_TOOL_NAMES.contains(&tool_name) {
                DispatchAction::Execute
            } else {
                DispatchAction::ApprovalRequired(tool_name.to_string())
            }
        }
    }
}

#[derive(Default)]
pub struct XmlToolDispatcher;

impl XmlToolDispatcher {
    fn parse_xml_tool_calls(response: &str) -> (String, Vec<ParsedToolCall>) {
        let mut text_parts = Vec::new();
        let mut calls = Vec::new();
        let mut remaining = response;

        while let Some(start) = remaining.find("<tool_call>") {
            let before = &remaining[..start];
            if !before.trim().is_empty() {
                text_parts.push(before.trim().to_string());
            }

            if let Some(end) = remaining[start..].find("</tool_call>") {
                let inner = &remaining[start + 11..start + end];
                match serde_json::from_str::<Value>(inner.trim()) {
                    Ok(parsed) => {
                        let name = parsed
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        if name.is_empty() {
                            remaining = &remaining[start + end + 12..];
                            continue;
                        }
                        let arguments = parsed
                            .get("arguments")
                            .cloned()
                            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
                        calls.push(ParsedToolCall {
                            name,
                            arguments,
                            tool_call_id: None,
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Malformed <tool_call> JSON: {e}");
                    }
                }
                remaining = &remaining[start + end + 12..];
            } else {
                break;
            }
        }

        if !remaining.trim().is_empty() {
            text_parts.push(remaining.trim().to_string());
        }

        (text_parts.join("\n"), calls)
    }

    pub fn tool_specs(tools: &[Box<dyn Tool>]) -> Vec<ToolSpec> {
        tools.iter().map(|tool| tool.spec()).collect()
    }
}

impl ToolDispatcher for XmlToolDispatcher {
    fn parse_response(&self, response: &ChatResponse) -> (String, Vec<ParsedToolCall>) {
        let text = response.text_or_empty();
        Self::parse_xml_tool_calls(text)
    }

    fn format_results(&self, results: &[ToolExecutionResult]) -> ConversationMessage {
        let mut content = String::new();
        for result in results {
            let status = if result.success { "ok" } else { "error" };
            let _ = writeln!(
                content,
                "<tool_result name=\"{}\" status=\"{}\">\n{}\n</tool_result>",
                result.name, status, result.output
            );
        }
        ConversationMessage::Chat(ChatMessage::user(format!("[Tool results]\n{content}")))
    }

    fn prompt_instructions(&self, tools: &[Box<dyn Tool>]) -> String {
        let mut instructions = String::new();
        instructions.push_str("## Tool Use Protocol\n\n");
        instructions
            .push_str("To use a tool, wrap a JSON object in <tool_call></tool_call> tags:\n\n");
        instructions.push_str(
            "```\n<tool_call>\n{\"name\": \"tool_name\", \"arguments\": {\"param\": \"value\"}}\n</tool_call>\n```\n\n",
        );
        instructions.push_str("### Available Tools\n\n");

        for tool in tools {
            let _ = writeln!(
                instructions,
                "- **{}**: {}\n  Parameters: `{}`",
                tool.name(),
                tool.description(),
                tool.parameters_schema()
            );
        }

        instructions
    }

    fn to_provider_messages(&self, history: &[ConversationMessage]) -> Vec<ChatMessage> {
        history
            .iter()
            .flat_map(|msg| match msg {
                ConversationMessage::Chat(chat) => vec![chat.clone()],
                ConversationMessage::AssistantToolCalls { text, .. } => {
                    vec![ChatMessage::assistant(text.clone().unwrap_or_default())]
                }
                ConversationMessage::ToolResults(results) => {
                    let mut content = String::new();
                    for result in results {
                        let _ = writeln!(
                            content,
                            "<tool_result id=\"{}\">\n{}\n</tool_result>",
                            result.tool_call_id, result.content
                        );
                    }
                    vec![ChatMessage::user(format!("[Tool results]\n{content}"))]
                }
            })
            .collect()
    }

    fn should_send_tool_specs(&self) -> bool {
        false
    }
}

pub struct NativeToolDispatcher;

impl ToolDispatcher for NativeToolDispatcher {
    fn parse_response(&self, response: &ChatResponse) -> (String, Vec<ParsedToolCall>) {
        let text = response.text.clone().unwrap_or_default();
        let calls = response
            .tool_calls
            .iter()
            .map(|tc| ParsedToolCall {
                name: tc.name.clone(),
                arguments: serde_json::from_str(&tc.arguments).unwrap_or_else(|e| {
                    tracing::warn!(
                        tool = %tc.name,
                        error = %e,
                        "Failed to parse native tool call arguments as JSON; defaulting to empty object"
                    );
                    Value::Object(serde_json::Map::new())
                }),
                tool_call_id: Some(tc.id.clone()),
            })
            .collect();
        (text, calls)
    }

    fn format_results(&self, results: &[ToolExecutionResult]) -> ConversationMessage {
        let messages = results
            .iter()
            .map(|result| ToolResultMessage {
                tool_call_id: result
                    .tool_call_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                content: result.output.clone(),
            })
            .collect();
        ConversationMessage::ToolResults(messages)
    }

    fn prompt_instructions(&self, _tools: &[Box<dyn Tool>]) -> String {
        String::new()
    }

    fn to_provider_messages(&self, history: &[ConversationMessage]) -> Vec<ChatMessage> {
        history
            .iter()
            .flat_map(|msg| match msg {
                ConversationMessage::Chat(chat) => vec![chat.clone()],
                ConversationMessage::AssistantToolCalls { text, tool_calls } => {
                    let payload = serde_json::json!({
                        "content": text,
                        "tool_calls": tool_calls,
                    });
                    vec![ChatMessage::assistant(payload.to_string())]
                }
                ConversationMessage::ToolResults(results) => results
                    .iter()
                    .map(|result| {
                        ChatMessage::tool(
                            serde_json::json!({
                                "tool_call_id": result.tool_call_id,
                                "content": result.content,
                            })
                            .to_string(),
                        )
                    })
                    .collect(),
            })
            .collect()
    }

    fn should_send_tool_specs(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_dispatcher_parses_tool_calls() {
        let response = ChatResponse {
            text: Some(
                "Checking\n<tool_call>{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}</tool_call>"
                    .into(),
            ),
            tool_calls: vec![],
        };
        let dispatcher = XmlToolDispatcher;
        let (_, calls) = dispatcher.parse_response(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "shell");
    }

    #[test]
    fn native_dispatcher_roundtrip() {
        let response = ChatResponse {
            text: Some("ok".into()),
            tool_calls: vec![crate::providers::ToolCall {
                id: "tc1".into(),
                name: "file_read".into(),
                arguments: "{\"path\":\"a.txt\"}".into(),
            }],
        };
        let dispatcher = NativeToolDispatcher;
        let (_, calls) = dispatcher.parse_response(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_call_id.as_deref(), Some("tc1"));

        let msg = dispatcher.format_results(&[ToolExecutionResult {
            name: "file_read".into(),
            output: "hello".into(),
            success: true,
            tool_call_id: Some("tc1".into()),
            action: DispatchAction::Execute,
        }]);
        match msg {
            ConversationMessage::ToolResults(results) => {
                assert_eq!(results.len(), 1);
                assert_eq!(results[0].tool_call_id, "tc1");
            }
            _ => panic!("expected tool results"),
        }
    }

    #[test]
    fn xml_format_results_contains_tool_result_tags() {
        let dispatcher = XmlToolDispatcher;
        let msg = dispatcher.format_results(&[ToolExecutionResult {
            name: "shell".into(),
            output: "ok".into(),
            success: true,
            tool_call_id: None,
            action: DispatchAction::Execute,
        }]);
        let rendered = match msg {
            ConversationMessage::Chat(chat) => chat.content,
            _ => String::new(),
        };
        assert!(rendered.contains("<tool_result"));
        assert!(rendered.contains("shell"));
    }

    #[test]
    fn native_format_results_keeps_tool_call_id() {
        let dispatcher = NativeToolDispatcher;
        let msg = dispatcher.format_results(&[ToolExecutionResult {
            name: "shell".into(),
            output: "ok".into(),
            success: true,
            tool_call_id: Some("tc-1".into()),
            action: DispatchAction::Execute,
        }]);

        match msg {
            ConversationMessage::ToolResults(results) => {
                assert_eq!(results.len(), 1);
                assert_eq!(results[0].tool_call_id, "tc-1");
            }
            _ => panic!("expected ToolResults variant"),
        }
    }

    #[test]
    fn test_risk_classification() {
        let dispatcher = NativeToolDispatcher;
        let action = dispatcher.check_tool_risk("shell", &serde_json::json!({"command": "rm -rf"}));
        assert_eq!(action, DispatchAction::ApprovalRequired("shell".into()));

        let safe_action = dispatcher.check_tool_risk("echo", &serde_json::json!({"message": "hi"}));
        assert_eq!(safe_action, DispatchAction::Execute);

        let unknown_action = dispatcher.check_tool_risk("unknown_tool", &serde_json::json!({}));
        assert_eq!(
            unknown_action,
            DispatchAction::ApprovalRequired("unknown_tool".into())
        );

        let mcp_action = dispatcher.check_tool_risk("mcp.docs.search", &serde_json::json!({}));
        match mcp_action {
            DispatchAction::ApprovalRequired(reason) => {
                assert!(reason.contains("mcp tool 'mcp.docs.search'"));
            }
            DispatchAction::Execute | DispatchAction::Blocked { .. } => {
                panic!("mcp tools must require approval")
            }
        }
    }

    #[test]
    fn plan_mode_denials_become_blocked_actions_without_changing_standard_semantics() {
        let mut policy = SecurityPolicy::default();
        policy.execution_mode = crate::config::ExecutionMode::Plan;

        assert_eq!(
            evaluate_tool_risk_with_policy("file_read", &policy),
            DispatchAction::Execute
        );

        match evaluate_tool_risk_with_policy("file_write", &policy) {
            DispatchAction::Blocked { code, reason } => {
                assert_eq!(code, crate::security::PLAN_MODE_BLOCKED_CODE);
                assert!(reason.contains("Plan Mode allows analysis-only capabilities"));
            }
            other => panic!("expected blocked plan-mode action, got {other:?}"),
        }

        assert_eq!(
            evaluate_tool_risk("shell"),
            DispatchAction::ApprovalRequired("shell".into())
        );
    }

    #[test]
    fn mission_origin_risk_classification_matches_standard_path() {
        assert_eq!(
            evaluate_tool_risk_for_origin("shell", ExecutionOrigin::Mission),
            evaluate_tool_risk_for_origin("shell", ExecutionOrigin::Standard)
        );
        assert_eq!(
            evaluate_tool_risk_for_origin("echo", ExecutionOrigin::Mission),
            DispatchAction::Execute
        );
    }
}
