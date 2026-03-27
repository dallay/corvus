use corvus::config::{McpConfig, McpServerConfig};
use corvus::tools::mcp;
use std::collections::BTreeMap;

fn mock_server(name: &str, payload: &str, capabilities: Vec<String>) -> McpServerConfig {
    McpServerConfig {
        name: name.to_string(),
        enabled: true,
        command: "__mcp_mock__".to_string(),
        args: vec![payload.to_string()],
        env: BTreeMap::new(),
        startup_timeout_ms: 50,
        call_timeout_ms: 500,
        output_limit_bytes: 1024,
        capabilities,
        resource_output_limit_bytes: None,
        prompt_output_limit_bytes: None,
    }
}

fn mock_config(servers: Vec<McpServerConfig>) -> McpConfig {
    McpConfig {
        enabled: true,
        servers,
    }
}

// ── Task 3.2: Cross-capability collision detection ──────────────

/// Tool `mcp.docs.search` and resource `mcp.docs.resource.search` coexist
/// without collision because the `.resource.` segment disambiguates them.
#[test]
fn tool_and_resource_same_name_coexist_without_collision() {
    let payload = r#"{
      "tools": [{"name":"search","description":"Search tool"}],
      "resources": [{"name":"search","uri":"docs://search","description":"Search resource"}]
    }"#;
    let server = mock_server(
        "docs",
        payload,
        vec!["tools".to_string(), "resources".to_string()],
    );
    let config = mock_config(vec![server]);

    let result = mcp::discover_capabilities(&config);
    assert!(result.is_ok(), "tool + resource same name must not collide");
    let tools = result.unwrap();
    // Tool is registered as adapter; resource collision detection passes
    // (resource adapter registration is Phase 1 — collision check still runs)
    assert!(tools.iter().any(|t| t.name() == "mcp.docs.search"));
}

/// Tool, resource, and prompt all named `summarize` on the same server
/// must resolve to three distinct identifiers:
/// - `mcp.devtools.summarize` (tool)
/// - `mcp.devtools.resource.summarize` (resource)
/// - `mcp.devtools.prompt.summarize` (prompt)
#[test]
fn tool_resource_prompt_same_name_resolve_to_distinct_identifiers() {
    let payload = r#"{
      "tools": [{"name":"summarize","description":"Summarize tool"}],
      "resources": [{"name":"summarize","uri":"dt://summarize","description":"Summarize resource"}],
      "prompts": [{"name":"summarize","description":"Summarize prompt"}]
    }"#;
    let server = mock_server(
        "devtools",
        payload,
        vec![
            "tools".to_string(),
            "resources".to_string(),
            "prompts".to_string(),
        ],
    );
    let config = mock_config(vec![server]);

    let result = mcp::discover_capabilities(&config);
    assert!(
        result.is_ok(),
        "three capability types with same name must not collide: {:?}",
        result.err()
    );
    let tools = result.unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();

    // Tool and prompt adapters are registered; resource adapter is Phase 1
    assert!(
        names.contains(&"mcp.devtools.summarize"),
        "tool must be registered"
    );
    assert!(
        names.contains(&"mcp.devtools.prompt.summarize"),
        "prompt must be registered"
    );
    // Resource collision detection ran without error (distinct from tool/prompt)
}

/// Duplicate resource within a server is rejected deterministically.
#[test]
fn duplicate_resource_within_server_is_rejected() {
    let payload = r#"{
      "resources": [
        {"name":"index","uri":"docs://index","description":"Index"},
        {"name":"index","uri":"docs://index2","description":"Index dup"}
      ]
    }"#;
    let server = mock_server("docs", payload, vec!["resources".to_string()]);
    let config = mock_config(vec![server]);

    let result = mcp::discover_capabilities(&config);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("duplicate"),
        "error must mention duplicate: {err}"
    );
    assert!(
        err.contains("mcp.docs.resource.index"),
        "error must identify the colliding name: {err}"
    );
}

/// Duplicate prompt within a server is rejected deterministically.
#[test]
fn duplicate_prompt_within_server_is_rejected() {
    let payload = r#"{
      "prompts": [
        {"name":"review","description":"Review v1"},
        {"name":"review","description":"Review v2"}
      ]
    }"#;
    let server = mock_server("workflows", payload, vec!["prompts".to_string()]);
    let config = mock_config(vec![server]);

    let result = mcp::discover_capabilities(&config);
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("duplicate"),
        "error must mention duplicate: {err}"
    );
    assert!(
        err.contains("mcp.workflows.prompt.review"),
        "error must identify the colliding name: {err}"
    );
}

/// Cross-server same-name resources do not collide because the server
/// segment disambiguates them:
/// `mcp.server1.resource.index` vs `mcp.server2.resource.index`
#[test]
fn cross_server_same_name_resources_do_not_collide() {
    let payload1 = r#"{
      "resources": [{"name":"index","uri":"s1://index","description":"Index"}]
    }"#;
    let payload2 = r#"{
      "resources": [{"name":"index","uri":"s2://index","description":"Index"}]
    }"#;
    let server1 = mock_server("server1", payload1, vec!["resources".to_string()]);
    let server2 = mock_server("server2", payload2, vec!["resources".to_string()]);
    let config = mock_config(vec![server1, server2]);

    let result = mcp::discover_capabilities(&config);
    assert!(
        result.is_ok(),
        "cross-server same-name resources must not collide: {:?}",
        result.err()
    );
}

/// Cross-server same-name prompts do not collide.
#[test]
fn cross_server_same_name_prompts_do_not_collide() {
    let payload1 = r#"{
      "prompts": [{"name":"review","description":"Review"}]
    }"#;
    let payload2 = r#"{
      "prompts": [{"name":"review","description":"Review"}]
    }"#;
    let server1 = mock_server("alpha", payload1, vec!["prompts".to_string()]);
    let server2 = mock_server("beta", payload2, vec!["prompts".to_string()]);
    let config = mock_config(vec![server1, server2]);

    let result = mcp::discover_capabilities(&config);
    assert!(result.is_ok());
    let tools = result.unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(names.contains(&"mcp.alpha.prompt.review"));
    assert!(names.contains(&"mcp.beta.prompt.review"));
}
