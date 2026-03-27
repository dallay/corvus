use crate::tools::traits::ToolSourceMetadata;
use serde_json::Value;

pub const CEREBRO_TOOL_STORE: &str = "mem_save";
pub const CEREBRO_TOOL_RECALL: &str = "mem_search";
pub const CEREBRO_TOOL_FORGET: &str = "mem_delete";

pub fn legacy_alias_target(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "memory_store" => Some(CEREBRO_TOOL_STORE),
        "memory_recall" => Some(CEREBRO_TOOL_RECALL),
        "memory_forget" => Some(CEREBRO_TOOL_FORGET),
        _ => None,
    }
}

pub fn normalize_legacy_recall_output(raw_output: &str) -> anyhow::Result<String> {
    let value: Value = serde_json::from_str(raw_output)
        .map_err(|err| anyhow::anyhow!("invalid Cerebro response: {err}"))?;
    let results = value
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("Cerebro response missing results"))?;

    if results.is_empty() {
        return Ok("No memories found matching that query.".to_string());
    }

    let mut output = format!("Found {} memories:\n", results.len());
    for entry in results {
        let summary = entry
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let key = entry
            .get("topic_key")
            .or_else(|| entry.get("memory_id"))
            .and_then(Value::as_str)
            .unwrap_or("memory");
        let _ =
            std::fmt::Write::write_fmt(&mut output, format_args!("- [cerebro] {key}: {summary}\n"));
    }

    Ok(output)
}

pub fn normalize_legacy_forget_output(raw_output: &str, key: &str) -> anyhow::Result<String> {
    let value: Value = serde_json::from_str(raw_output)
        .map_err(|err| anyhow::anyhow!("invalid Cerebro response: {err}"))?;
    let deleted = value
        .get("deleted")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if deleted {
        Ok(format!("Forgot memory: {key}"))
    } else {
        Ok(format!("No memory found with key: {key}"))
    }
}

pub fn normalize_tool_name(server: &str, tool_name: &str) -> anyhow::Result<String> {
    validate_identifier("server", server)?;
    validate_identifier("tool", tool_name)?;
    Ok(format!("mcp.{server}.{tool_name}"))
}

pub fn source_metadata(server: &str, tool_name: &str) -> ToolSourceMetadata {
    ToolSourceMetadata {
        kind: "mcp".to_string(),
        provider: Some("mcp".to_string()),
        server: Some(server.to_string()),
        original_name: Some(tool_name.to_string()),
    }
}

pub fn normalize_resource_name(server: &str, resource_name: &str) -> anyhow::Result<String> {
    validate_identifier("server", server)?;
    validate_identifier("resource", resource_name)?;
    Ok(format!("mcp.{server}.resource.{resource_name}"))
}

pub fn normalize_prompt_name(server: &str, prompt_name: &str) -> anyhow::Result<String> {
    validate_identifier("server", server)?;
    validate_identifier("prompt", prompt_name)?;
    Ok(format!("mcp.{server}.prompt.{prompt_name}"))
}

pub fn source_metadata_resource(server: &str, resource_name: &str) -> ToolSourceMetadata {
    ToolSourceMetadata {
        kind: "mcp_resource".to_string(),
        provider: Some("mcp".to_string()),
        server: Some(server.to_string()),
        original_name: Some(resource_name.to_string()),
    }
}

pub fn source_metadata_prompt(server: &str, prompt_name: &str) -> ToolSourceMetadata {
    ToolSourceMetadata {
        kind: "mcp_prompt".to_string(),
        provider: Some("mcp".to_string()),
        server: Some(server.to_string()),
        original_name: Some(prompt_name.to_string()),
    }
}

fn validate_identifier(kind: &str, value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        anyhow::bail!("MCP {kind} identifier must be non-empty");
    }

    if value.eq_ignore_ascii_case("mcp")
        || value.eq_ignore_ascii_case("resource")
        || value.eq_ignore_ascii_case("prompt")
    {
        anyhow::bail!("MCP {kind} identifier '{value}' is reserved");
    }

    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        anyhow::bail!("MCP {kind} identifier contains invalid characters");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_name_uses_mcp_server_tool_format() {
        let canonical = normalize_tool_name("docs", "search").unwrap();
        assert_eq!(canonical, "mcp.docs.search");
    }

    #[test]
    fn reserved_identifier_mcp_is_rejected() {
        let err = normalize_tool_name("mcp", "search")
            .unwrap_err()
            .to_string();
        assert!(err.contains("reserved"));
    }

    // ── Resource normalization ───────────────────────────────

    #[test]
    fn normalize_resource_name_produces_canonical_format() {
        let name = normalize_resource_name("docs", "api-spec").unwrap();
        assert_eq!(name, "mcp.docs.resource.api-spec");
    }

    #[test]
    fn normalize_resource_name_rejects_empty_server() {
        assert!(normalize_resource_name("", "api-spec").is_err());
    }

    #[test]
    fn normalize_resource_name_rejects_empty_resource() {
        assert!(normalize_resource_name("docs", "").is_err());
    }

    // ── Prompt normalization ─────────────────────────────────

    #[test]
    fn normalize_prompt_name_produces_canonical_format() {
        let name = normalize_prompt_name("workflows", "code-review").unwrap();
        assert_eq!(name, "mcp.workflows.prompt.code-review");
    }

    #[test]
    fn normalize_prompt_name_rejects_empty_server() {
        assert!(normalize_prompt_name("", "code-review").is_err());
    }

    #[test]
    fn normalize_prompt_name_rejects_empty_prompt() {
        assert!(normalize_prompt_name("workflows", "").is_err());
    }

    // ── Reserved words ───────────────────────────────────────

    #[test]
    fn reserved_word_resource_rejected_as_server_name() {
        let err = normalize_tool_name("resource", "search")
            .unwrap_err()
            .to_string();
        assert!(err.contains("reserved"));
    }

    #[test]
    fn reserved_word_prompt_rejected_as_server_name() {
        let err = normalize_tool_name("prompt", "search")
            .unwrap_err()
            .to_string();
        assert!(err.contains("reserved"));
    }

    #[test]
    fn reserved_word_resource_rejected_as_tool_name() {
        let err = normalize_tool_name("docs", "resource")
            .unwrap_err()
            .to_string();
        assert!(err.contains("reserved"));
    }

    #[test]
    fn reserved_word_prompt_rejected_as_tool_name() {
        let err = normalize_tool_name("docs", "prompt")
            .unwrap_err()
            .to_string();
        assert!(err.contains("reserved"));
    }

    // ── Source metadata ──────────────────────────────────────

    #[test]
    fn source_metadata_resource_has_correct_kind() {
        let meta = source_metadata_resource("docs", "api-spec");
        assert_eq!(meta.kind, "mcp_resource");
        assert_eq!(meta.server.as_deref(), Some("docs"));
        assert_eq!(meta.original_name.as_deref(), Some("api-spec"));
    }

    #[test]
    fn source_metadata_prompt_has_correct_kind() {
        let meta = source_metadata_prompt("workflows", "code-review");
        assert_eq!(meta.kind, "mcp_prompt");
        assert_eq!(meta.server.as_deref(), Some("workflows"));
        assert_eq!(meta.original_name.as_deref(), Some("code-review"));
    }
}
