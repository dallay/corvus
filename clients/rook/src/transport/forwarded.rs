use axum::http::HeaderMap;
use ipnet::IpNet;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use crate::config::TrustedProxyConfig;
use crate::transport::context::{ForwardedTrust, SanitizedForwardedContext};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardedResolution {
    pub context: SanitizedForwardedContext,
    pub forwarded_present: bool,
}

pub fn resolve_forwarded_context(
    headers: &HeaderMap,
    direct_peer_addr: Option<SocketAddr>,
    config: &TrustedProxyConfig,
) -> ForwardedResolution {
    let mut context = SanitizedForwardedContext::default();
    let forwarded_present = has_forwarded_metadata(headers);

    if !forwarded_present {
        return ForwardedResolution {
            context,
            forwarded_present: false,
        };
    }

    if !should_trust_forwarded_headers(direct_peer_addr, config) {
        context.trust = ForwardedTrust::Ignored;
        return ForwardedResolution {
            context,
            forwarded_present: true,
        };
    }

    let (trusted_any, malformed_any) =
        apply_allowed_forwarded_headers(headers, config, &mut context);
    context.trust = resolve_forwarded_trust(trusted_any, malformed_any, forwarded_present);

    ForwardedResolution {
        context,
        forwarded_present: true,
    }
}

fn should_trust_forwarded_headers(
    direct_peer_addr: Option<SocketAddr>,
    config: &TrustedProxyConfig,
) -> bool {
    if !config.enabled {
        return false;
    }

    let Some(peer_addr) = direct_peer_addr else {
        return false;
    };

    peer_matches_trusted_proxy(peer_addr.ip(), &config.trusted_cidrs)
}

fn apply_allowed_forwarded_headers(
    headers: &HeaderMap,
    config: &TrustedProxyConfig,
    context: &mut SanitizedForwardedContext,
) -> (bool, bool) {
    let mut trusted_any = false;
    let mut malformed_any = false;

    apply_forwarded_value(
        headers,
        config.allowed_headers.x_forwarded_for,
        "x-forwarded-for",
        context,
        &mut trusted_any,
        &mut malformed_any,
        parse_ip_header,
        |ctx, value| ctx.client_ip = Some(value),
    );
    apply_forwarded_value(
        headers,
        config.allowed_headers.x_real_ip,
        "x-real-ip",
        context,
        &mut trusted_any,
        &mut malformed_any,
        parse_ip_header,
        |ctx, value| {
            if ctx.client_ip.is_none() {
                ctx.client_ip = Some(value);
            }
        },
    );
    apply_forwarded_value(
        headers,
        config.allowed_headers.x_forwarded_host,
        "x-forwarded-host",
        context,
        &mut trusted_any,
        &mut malformed_any,
        parse_visible_header,
        |ctx, value| ctx.host = Some(value),
    );
    apply_forwarded_value(
        headers,
        config.allowed_headers.x_forwarded_proto,
        "x-forwarded-proto",
        context,
        &mut trusted_any,
        &mut malformed_any,
        parse_proto_header,
        |ctx, value| ctx.proto = Some(value),
    );
    apply_forwarded_value(
        headers,
        config.allowed_headers.x_forwarded_port,
        "x-forwarded-port",
        context,
        &mut trusted_any,
        &mut malformed_any,
        parse_port_header,
        |ctx, value| ctx.port = Some(value),
    );

    (trusted_any, malformed_any)
}

#[allow(clippy::too_many_arguments)]
fn apply_forwarded_value<T>(
    headers: &HeaderMap,
    enabled: bool,
    header_name: &'static str,
    context: &mut SanitizedForwardedContext,
    trusted_any: &mut bool,
    malformed_any: &mut bool,
    parser: impl Fn(&HeaderMap, &str) -> Option<T>,
    on_value: impl Fn(&mut SanitizedForwardedContext, T),
) {
    if !enabled || !headers.contains_key(header_name) {
        return;
    }

    match parser(headers, header_name) {
        Some(value) => {
            on_value(context, value);
            *trusted_any = true;
        }
        None => {
            *malformed_any = true;
            context.ignored_headers.push(header_name);
        }
    }
}

fn resolve_forwarded_trust(
    trusted_any: bool,
    malformed_any: bool,
    forwarded_present: bool,
) -> ForwardedTrust {
    if trusted_any {
        ForwardedTrust::Trusted
    } else if malformed_any || forwarded_present {
        ForwardedTrust::Ignored
    } else {
        ForwardedTrust::Absent
    }
}

fn has_forwarded_metadata(headers: &HeaderMap) -> bool {
    [
        "forwarded",
        "x-forwarded-for",
        "x-forwarded-host",
        "x-forwarded-proto",
        "x-forwarded-port",
        "x-real-ip",
        "via",
    ]
    .iter()
    .any(|header| headers.contains_key(*header))
}

fn peer_matches_trusted_proxy(peer_ip: IpAddr, trusted_cidrs: &[String]) -> bool {
    trusted_cidrs.iter().any(|cidr| {
        IpNet::from_str(cidr)
            .map(|network| network.contains(&peer_ip))
            .unwrap_or(false)
    })
}

fn single_header_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?.trim();
    if values.next().is_some() || value.is_empty() {
        return None;
    }
    Some(value)
}

fn parse_visible_header(headers: &HeaderMap, name: &str) -> Option<String> {
    let value = single_header_value(headers, name)?;
    if value.chars().all(|character| character.is_ascii_graphic()) {
        Some(value.to_string())
    } else {
        None
    }
}

fn parse_ip_header(headers: &HeaderMap, name: &str) -> Option<IpAddr> {
    let value = single_header_value(headers, name)?;
    let candidate = value.split(',').next()?.trim();
    candidate.parse().ok()
}

fn parse_proto_header(headers: &HeaderMap, name: &str) -> Option<String> {
    let value = single_header_value(headers, name)?;
    match value {
        "http" | "https" => Some(value.to_string()),
        _ => None,
    }
}

fn parse_port_header(headers: &HeaderMap, name: &str) -> Option<u16> {
    let value = single_header_value(headers, name)?;
    let port: u16 = value.parse().ok()?;
    (port > 0).then_some(port)
}

#[cfg(test)]
mod tests {
    use super::resolve_forwarded_context;
    use crate::config::{TrustedForwardedHeaders, TrustedProxyConfig};
    use crate::transport::context::ForwardedTrust;
    use axum::http::HeaderMap;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn trusted_proxy_config() -> TrustedProxyConfig {
        TrustedProxyConfig {
            enabled: true,
            trusted_cidrs: vec!["127.0.0.0/8".to_string()],
            allowed_headers: TrustedForwardedHeaders {
                x_forwarded_for: true,
                x_forwarded_host: true,
                x_forwarded_proto: true,
                x_forwarded_port: true,
                x_real_ip: true,
                ..TrustedForwardedHeaders::default()
            },
        }
    }

    #[test]
    fn disabled_trust_policy_ignores_forwarded_metadata() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.9".parse().unwrap());

        let resolution = resolve_forwarded_context(&headers, None, &TrustedProxyConfig::default());

        assert!(resolution.forwarded_present);
        assert_eq!(resolution.context.trust, ForwardedTrust::Ignored);
        assert_eq!(resolution.context.client_ip, None);
    }

    #[test]
    fn enabled_policy_without_peer_address_falls_back_to_strict_default() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", "https".parse().unwrap());

        let resolution = resolve_forwarded_context(&headers, None, &trusted_proxy_config());

        assert_eq!(resolution.context.trust, ForwardedTrust::Ignored);
        assert_eq!(resolution.context.proto, None);
    }

    #[test]
    fn untrusted_source_ignores_forwarded_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-host", "public.example.com".parse().unwrap());

        let resolution = resolve_forwarded_context(
            &headers,
            Some(SocketAddr::from((Ipv4Addr::new(10, 0, 0, 5), 8080))),
            &trusted_proxy_config(),
        );

        assert_eq!(resolution.context.trust, ForwardedTrust::Ignored);
        assert_eq!(resolution.context.host, None);
    }

    #[test]
    fn trusted_source_adopts_allowed_forwarded_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.9".parse().unwrap());
        headers.insert("x-forwarded-host", "public.example.com".parse().unwrap());
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        headers.insert("x-forwarded-port", "443".parse().unwrap());

        let resolution = resolve_forwarded_context(
            &headers,
            Some(SocketAddr::from((Ipv4Addr::LOCALHOST, 8080))),
            &trusted_proxy_config(),
        );

        assert_eq!(resolution.context.trust, ForwardedTrust::Trusted);
        assert_eq!(
            resolution.context.client_ip,
            Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9)))
        );
        assert_eq!(
            resolution.context.host.as_deref(),
            Some("public.example.com")
        );
        assert_eq!(resolution.context.proto.as_deref(), Some("https"));
        assert_eq!(resolution.context.port, Some(443));
    }

    #[test]
    fn malformed_values_are_ignored_and_tracked() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "not-an-ip".parse().unwrap());
        headers.insert("x-forwarded-port", "0".parse().unwrap());

        let resolution = resolve_forwarded_context(
            &headers,
            Some(SocketAddr::from((Ipv4Addr::LOCALHOST, 8080))),
            &trusted_proxy_config(),
        );

        assert_eq!(resolution.context.trust, ForwardedTrust::Ignored);
        assert_eq!(resolution.context.client_ip, None);
        assert_eq!(resolution.context.port, None);
        assert!(resolution
            .context
            .ignored_headers
            .contains(&"x-forwarded-for"));
        assert!(resolution
            .context
            .ignored_headers
            .contains(&"x-forwarded-port"));
    }

    #[test]
    fn via_is_diagnostic_only_even_for_trusted_sources() {
        let mut headers = HeaderMap::new();
        headers.insert("via", "1.1 proxy.example.com".parse().unwrap());

        let resolution = resolve_forwarded_context(
            &headers,
            Some(SocketAddr::from((Ipv4Addr::LOCALHOST, 8080))),
            &trusted_proxy_config(),
        );

        assert!(resolution.forwarded_present);
        assert_eq!(resolution.context.client_ip, None);
        assert_eq!(resolution.context.host, None);
        assert_eq!(resolution.context.proto, None);
        assert_eq!(resolution.context.port, None);
    }
}
