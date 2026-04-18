use super::url_safety::{extract_host, host_matches_allowlist, normalize_allowed_domains};
use anyhow::bail;
use futures_util::StreamExt;
use std::time::Duration;

pub(crate) fn validate_outbound_url(
    raw_url: &str,
    allowed_domains: &[String],
    allowlist_field: &str,
) -> anyhow::Result<String> {
    let url = raw_url.trim();

    if url.is_empty() {
        bail!("URL cannot be empty");
    }

    if url.chars().any(char::is_whitespace) {
        bail!("URL cannot contain whitespace");
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        bail!("Only http:// and https:// URLs are allowed");
    }

    if allowed_domains.is_empty() {
        bail!(
            "HTTP request tool is enabled but no allowed_domains are configured. Add [{allowlist_field}].allowed_domains in config.toml"
        );
    }

    let host = extract_host(
        url,
        &["http://", "https://"],
        "Only http:// and https:// URLs are allowed",
        allowlist_field,
    )?;

    if is_private_or_local_host(&host) {
        bail!("Blocked local/private host: {host}");
    }

    if !host_matches_allowlist(&host, allowed_domains) {
        bail!("Host '{host}' is not in {allowlist_field}.allowed_domains");
    }

    Ok(url.to_string())
}

pub(crate) fn normalized_allowed_domains(domains: Vec<String>) -> Vec<String> {
    normalize_allowed_domains(domains)
}

pub(crate) fn build_read_only_client(timeout_secs: u64) -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(reqwest::redirect::Policy::none())
        .build()?)
}

pub(crate) async fn execute_get(url: &str, timeout_secs: u64) -> anyhow::Result<reqwest::Response> {
    let client = build_read_only_client(timeout_secs)?;
    Ok(client.get(url).send().await?)
}

pub(crate) async fn read_response_body_limited(
    response: reqwest::Response,
    max_response_size: usize,
) -> anyhow::Result<(Vec<u8>, bool)> {
    let mut body = Vec::new();
    let mut truncated = false;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;

        let remaining = max_response_size.saturating_sub(body.len());
        if remaining == 0 {
            truncated = true;
            break;
        }

        if chunk.len() > remaining {
            body.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }

        body.extend_from_slice(&chunk);
    }

    Ok((body, truncated))
}

pub(crate) fn is_private_or_local_host(host: &str) -> bool {
    let bare = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);

    let has_local_tld = bare
        .rsplit('.')
        .next()
        .is_some_and(|label| label == "local");
    if bare.eq_ignore_ascii_case("localhost") || has_local_tld {
        return true;
    }

    if let Ok(ip) = bare.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => is_non_global_v4(v4),
            std::net::IpAddr::V6(v6) => is_non_global_v6(v6),
        };
    }

    false
}

fn is_non_global_v4(v4: std::net::Ipv4Addr) -> bool {
    let o = v4.octets();
    v4.is_private()
        || v4.is_loopback()
        || v4.is_link_local()
        || v4.is_unspecified()
        || v4.is_multicast()
        || v4.is_broadcast()
        || o[0] == 0
        || (o[0] == 100 && (64..=127).contains(&o[1]))
        || (o[0] == 198 && (o[1] == 18 || o[1] == 19))
        || (o[0] == 192 && o[1] == 0 && o[2] == 2)
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)
        || (o[0] == 203 && o[1] == 0 && o[2] == 113)
        || (240..=255).contains(&o[0])
}

fn is_non_global_v6(v6: std::net::Ipv6Addr) -> bool {
    let segs = v6.segments();
    v6.is_loopback()
        || v6.is_unspecified()
        || v6.is_multicast()
        || (segs[0] & 0xffc0) == 0xfe80
        || (segs[0] & 0xfe00) == 0xfc00
        || (segs[0] == 0x2001 && segs[1] == 0x0db8)
        || v6.to_ipv4_mapped().is_some_and(is_non_global_v4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_url_accepts_public_allowlisted_host() {
        let url = validate_outbound_url(
            "https://docs.example.com/page",
            &normalized_allowed_domains(vec!["example.com".into()]),
            "http_request",
        )
        .unwrap();
        assert_eq!(url, "https://docs.example.com/page");
    }

    #[test]
    fn validate_url_rejects_private_hosts_before_request() {
        let err = validate_outbound_url(
            "http://127.0.0.1:8080/admin",
            &normalized_allowed_domains(vec!["127.0.0.1".into()]),
            "http_request",
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("local/private"));
    }

    #[test]
    fn validate_url_rejects_unsupported_scheme() {
        let err = validate_outbound_url(
            "file:///etc/passwd",
            &normalized_allowed_domains(vec!["example.com".into()]),
            "http_request",
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("http:// and https://"));
    }
}
