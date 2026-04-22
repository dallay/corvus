use std::net::{IpAddr, SocketAddr};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteSurface {
    AdminApi,
    GatewayV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitedSurface {
    AdminApi,
    GatewayModels,
    GatewayChatCompletions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForwardedTrust {
    Absent,
    Ignored,
    Trusted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedForwardedContext {
    pub trust: ForwardedTrust,
    pub client_ip: Option<IpAddr>,
    pub host: Option<String>,
    pub proto: Option<String>,
    pub port: Option<u16>,
    pub ignored_headers: Vec<&'static str>,
}

impl Default for SanitizedForwardedContext {
    fn default() -> Self {
        Self {
            trust: ForwardedTrust::Absent,
            client_ip: None,
            host: None,
            proto: None,
            port: None,
            ignored_headers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedTransportContext {
    pub request_id: String,
    pub route_surface: RouteSurface,
    pub direct_peer_addr: Option<SocketAddr>,
    pub forwarded: SanitizedForwardedContext,
}
