pub(crate) fn normalize_allowed_domains(domains: Vec<String>) -> Vec<String> {
    let mut normalized = domains
        .into_iter()
        .filter_map(|domain| normalize_domain(&domain))
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized.dedup();
    normalized
}

pub(crate) fn normalize_domain(raw: &str) -> Option<String> {
    let mut domain = raw.trim().to_lowercase();
    if domain.is_empty() {
        return None;
    }

    if let Some(stripped) = domain.strip_prefix("https://") {
        domain = stripped.to_string();
    } else if let Some(stripped) = domain.strip_prefix("http://") {
        domain = stripped.to_string();
    }

    if let Some((host, _)) = domain.split_once('/') {
        domain = host.to_string();
    }

    domain = domain
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_string();

    if let Some((host, _)) = domain.split_once(':') {
        domain = host.to_string();
    }

    if domain.is_empty() || domain.chars().any(char::is_whitespace) {
        return None;
    }

    Some(domain)
}

pub(crate) fn extract_host(
    url: &str,
    accepted_schemes: &[&str],
    scheme_error: &str,
    ipv6_context: &str,
) -> anyhow::Result<String> {
    let parsed = Url::parse(url).map_err(|_| anyhow::anyhow!("Invalid URL"))?;
    let parsed_scheme = format!("{}://", parsed.scheme());
    if !accepted_schemes.contains(&parsed_scheme.as_str()) {
        anyhow::bail!(scheme_error.to_string());
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        anyhow::bail!("URL userinfo is not allowed");
    }

    match parsed.host() {
        Some(Host::Domain(domain)) => {
            let host = domain.trim_end_matches('.').to_lowercase();
            if host.is_empty() {
                anyhow::bail!("URL must include a valid host");
            }
            Ok(host)
        }
        Some(Host::Ipv4(ipv4)) => {
            let ip = IpAddr::from_str(&ipv4.to_string())?;
            Ok(ip.to_string())
        }
        Some(Host::Ipv6(_)) => {
            anyhow::bail!("IPv6 hosts are not supported in {ipv6_context}");
        }
        None => anyhow::bail!("URL must include a host"),
    }
}

pub(crate) fn host_matches_allowlist(host: &str, allowed_domains: &[String]) -> bool {
    allowed_domains.iter().any(|domain| {
        host == domain
            || host
                .strip_suffix(domain)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
}
use std::net::IpAddr;
use std::str::FromStr;
use url::{Host, Url};
