use url::{Host, Url};

pub(crate) fn normalize_allowed_domains(domains: Vec<String>) -> anyhow::Result<Vec<String>> {
    let mut normalized = Vec::new();
    for domain in domains {
        normalized.push(normalize_domain(&domain)?);
    }
    normalized.sort_unstable();
    normalized.dedup();
    Ok(normalized)
}

pub(crate) fn normalize_domain(raw: &str) -> anyhow::Result<String> {
    let mut domain = raw.trim().to_lowercase();
    if domain.is_empty() {
        anyhow::bail!("Domain cannot be empty");
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

    if domain.contains(':') {
        anyhow::bail!("Domain cannot contain a port: {}", domain);
    }

    if domain.is_empty() || domain.chars().any(char::is_whitespace) {
        anyhow::bail!("Invalid domain: {}", domain);
    }

    Ok(domain)
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
        Some(Host::Ipv4(ipv4)) => Ok(ipv4.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_domain() {
        assert_eq!(normalize_domain("HTTPS://EXAMPLE.COM/path").unwrap(), "example.com".to_string());
        assert!(normalize_domain("example.com:8080").is_err());
        assert_eq!(normalize_domain("  .example.com.  ").unwrap(), "example.com".to_string());
        assert!(normalize_domain("example com").is_err());
        assert!(normalize_domain("").is_err());
    }

    #[test]
    fn test_normalize_allowed_domains() {
        let domains = vec!["example.com".into(), "EXAMPLE.COM".into(), "https://google.com/".into()];
        let normalized = normalize_allowed_domains(domains).unwrap();
        assert_eq!(normalized, vec!["example.com".to_string(), "google.com".to_string()]);

        let bad_domains = vec!["example.com".into(), "localhost:8080".into()];
        assert!(normalize_allowed_domains(bad_domains).is_err());
    }
}
