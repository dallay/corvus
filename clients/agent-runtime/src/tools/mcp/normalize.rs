use crate::tools::traits::ToolSourceMetadata;

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

fn validate_identifier(kind: &str, value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        anyhow::bail!("MCP {kind} identifier must be non-empty");
    }

    if value.eq_ignore_ascii_case("mcp") {
        anyhow::bail!("MCP {kind} identifier 'mcp' is reserved");
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
    fn reserved_identifier_is_rejected() {
        let err = normalize_tool_name("mcp", "search")
            .unwrap_err()
            .to_string();
        assert!(err.contains("reserved"));
    }
}
