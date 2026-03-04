use corvus::agent::dispatcher::{evaluate_tool_risk, DispatchAction};
use corvus::approval::structured_denial_payload;

#[test]
fn mcp_tools_are_deny_by_default_in_dispatcher() {
    match evaluate_tool_risk("mcp.docs.search") {
        DispatchAction::ApprovalRequired(reason) => {
            assert!(reason.contains("requires explicit approval"));
        }
        DispatchAction::Execute => panic!("mcp.* must never execute without approval"),
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
