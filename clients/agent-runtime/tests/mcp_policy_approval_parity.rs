use corvus::agent::dispatcher::{
    evaluate_tool_risk, evaluate_tool_risk_for_origin, DispatchAction,
};
use corvus::approval::structured_denial_payload;
use corvus::security::ExecutionOrigin;

#[test]
fn mcp_tools_are_deny_by_default_in_dispatcher() {
    match evaluate_tool_risk("mcp.docs.search") {
        DispatchAction::ApprovalRequired(reason) => {
            assert!(reason.contains("requires explicit approval"));
        }
        DispatchAction::Execute | DispatchAction::Blocked { .. } => {
            panic!("mcp.* must never execute without approval")
        }
    }
}

#[test]
fn unknown_and_high_risk_tools_require_approval() {
    assert!(matches!(
        evaluate_tool_risk("unknown_tool"),
        DispatchAction::ApprovalRequired(_)
    ));
    assert!(matches!(
        evaluate_tool_risk("shell"),
        DispatchAction::ApprovalRequired(_)
    ));
}

#[test]
fn structured_denial_payload_is_stable_for_cross_entrypoint_use() {
    let denial = structured_denial_payload(
        "mcp.docs.search",
        "mcp tool 'mcp.docs.search' requires explicit approval",
    );

    assert_eq!(denial["code"], "approval_required");
    assert_eq!(denial["tool"], "mcp.docs.search");
    assert!(denial["reason"]
        .as_str()
        .unwrap_or_default()
        .contains("explicit approval"));
}

// ── Task 3.1: Entry-point parity for resources and prompts ──────

#[test]
fn mcp_resource_requires_approval_in_dispatcher() {
    match evaluate_tool_risk("mcp.docs.resource.api-spec") {
        DispatchAction::ApprovalRequired(reason) => {
            assert!(reason.contains("requires explicit approval"));
        }
        DispatchAction::Execute | DispatchAction::Blocked { .. } => {
            panic!("mcp resource must require approval")
        }
    }
}

#[test]
fn mcp_prompt_requires_approval_in_dispatcher() {
    match evaluate_tool_risk("mcp.workflows.prompt.code-review") {
        DispatchAction::ApprovalRequired(reason) => {
            assert!(reason.contains("requires explicit approval"));
        }
        DispatchAction::Execute | DispatchAction::Blocked { .. } => {
            panic!("mcp prompt must require approval")
        }
    }
}

#[test]
fn mcp_resource_parity_across_standard_and_mission_origins() {
    let resource = "mcp.docs.resource.api-spec";
    let standard = evaluate_tool_risk_for_origin(resource, ExecutionOrigin::Standard);
    let mission = evaluate_tool_risk_for_origin(resource, ExecutionOrigin::Mission);
    assert_eq!(
        standard, mission,
        "resource policy must be identical across origins"
    );
    assert!(matches!(standard, DispatchAction::ApprovalRequired(_)));
}

#[test]
fn mcp_prompt_parity_across_standard_and_mission_origins() {
    let prompt = "mcp.workflows.prompt.code-review";
    let standard = evaluate_tool_risk_for_origin(prompt, ExecutionOrigin::Standard);
    let mission = evaluate_tool_risk_for_origin(prompt, ExecutionOrigin::Mission);
    assert_eq!(
        standard, mission,
        "prompt policy must be identical across origins"
    );
    assert!(matches!(standard, DispatchAction::ApprovalRequired(_)));
}
