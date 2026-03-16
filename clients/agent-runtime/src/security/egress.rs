use crate::config::MemoryCerebroConfig;
use crate::security::policy::ToolOperation;
use url::Url;

pub fn enforce_cerebro_egress(
    endpoint: &str,
    config: &MemoryCerebroConfig,
    _operation: ToolOperation,
) -> anyhow::Result<()> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        anyhow::bail!("memory.cerebro.endpoint must be non-empty when configured");
    }

    let parsed = Url::parse(endpoint)
        .map_err(|_| anyhow::anyhow!("memory.cerebro.endpoint is not a valid URL"))?;
    let scheme = parsed.scheme();
    let is_insecure = matches!(scheme, "http" | "ws");
    let is_secure = matches!(scheme, "https" | "wss");

    if !is_insecure && !is_secure {
        anyhow::bail!("memory.cerebro.endpoint must use http, https, ws, or wss transport");
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("memory.cerebro.endpoint must include a host"))?;

    if is_insecure && !config.allow_insecure_loopback {
        anyhow::bail!("memory.cerebro.endpoint requires allow_insecure_loopback for http/ws");
    }

    if is_insecure && config.allow_insecure_loopback && !is_loopback_host(host) {
        anyhow::bail!(
            "memory.cerebro.endpoint allows insecure transport only for loopback addresses"
        );
    }

    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    let trimmed = host.trim().trim_matches('[').trim_matches(']');
    if trimmed.eq_ignore_ascii_case("localhost") {
        return true;
    }
    trimmed
        .parse::<std::net::IpAddr>()
        .is_ok_and(|addr| addr.is_loopback())
}
