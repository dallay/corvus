use axum::http::{HeaderMap, HeaderName, HeaderValue};
use uuid::Uuid;

use crate::config::RequestIdConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestIdAdoption {
    Adopted(String),
    Generated(String),
}

impl RequestIdAdoption {
    pub fn effective(&self) -> &str {
        match self {
            Self::Adopted(value) | Self::Generated(value) => value,
        }
    }
}

pub fn resolve_request_id(headers: &HeaderMap, config: &RequestIdConfig) -> RequestIdAdoption {
    let header_name = HeaderName::from_bytes(config.inbound_header_name.as_bytes())
        .expect("request ID header name must be validated before runtime use");
    let values: Vec<_> = headers.get_all(&header_name).iter().collect();

    if values.len() == 1 {
        if let Some(candidate) = validate_request_id_value(values[0], config.max_length) {
            return RequestIdAdoption::Adopted(candidate);
        }
    }

    RequestIdAdoption::Generated(Uuid::new_v4().hyphenated().to_string())
}

pub fn set_response_request_id_header(
    headers: &mut HeaderMap,
    request_id: &str,
    config: &RequestIdConfig,
) {
    let header_name = HeaderName::from_bytes(config.response_header_name.as_bytes())
        .expect("response request ID header name must be validated before runtime use");
    let header_value = HeaderValue::from_str(request_id)
        .expect("effective request ID must always be valid for response headers");
    headers.insert(header_name, header_value);
}

fn validate_request_id_value(value: &HeaderValue, max_length: usize) -> Option<String> {
    let text = value.to_str().ok()?.trim();
    if text.is_empty() || text.len() > max_length {
        return None;
    }

    if !text
        .chars()
        .all(|character| character.is_ascii_graphic() && character != ',')
    {
        return None;
    }

    Some(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::{resolve_request_id, set_response_request_id_header, RequestIdAdoption};
    use crate::config::RequestIdConfig;
    use axum::http::{HeaderMap, HeaderValue};

    fn config() -> RequestIdConfig {
        RequestIdConfig::default()
    }

    #[test]
    fn resolves_generated_request_id_when_header_absent() {
        let headers = HeaderMap::new();

        let request_id = resolve_request_id(&headers, &config());

        match request_id {
            RequestIdAdoption::Generated(value) => assert_eq!(value.len(), 36),
            other => panic!("expected generated request id, got {other:?}"),
        }
    }

    #[test]
    fn resolves_adopted_request_id_for_valid_inbound_value() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static("trace-123"));

        let request_id = resolve_request_id(&headers, &config());

        assert_eq!(
            request_id,
            RequestIdAdoption::Adopted("trace-123".to_string())
        );
    }

    #[test]
    fn resolves_generated_request_id_for_empty_inbound_value() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static(""));

        let request_id = resolve_request_id(&headers, &config());

        assert!(matches!(request_id, RequestIdAdoption::Generated(_)));
    }

    #[test]
    fn resolves_generated_request_id_for_whitespace_inbound_value() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static("   "));

        let request_id = resolve_request_id(&headers, &config());

        assert!(matches!(request_id, RequestIdAdoption::Generated(_)));
    }

    #[test]
    fn resolves_generated_request_id_for_malformed_inbound_value() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static("trace id"));

        let request_id = resolve_request_id(&headers, &config());

        assert!(matches!(request_id, RequestIdAdoption::Generated(_)));
    }

    #[test]
    fn resolves_generated_request_id_for_multi_value_input() {
        let mut headers = HeaderMap::new();
        headers.append("x-request-id", HeaderValue::from_static("trace-1"));
        headers.append("x-request-id", HeaderValue::from_static("trace-2"));

        let request_id = resolve_request_id(&headers, &config());

        assert!(matches!(request_id, RequestIdAdoption::Generated(_)));
    }

    #[test]
    fn resolves_generated_request_id_for_oversized_input() {
        let oversized = "a".repeat(129);
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_str(&oversized).unwrap());

        let request_id = resolve_request_id(&headers, &config());

        assert!(matches!(request_id, RequestIdAdoption::Generated(_)));
    }

    #[test]
    fn writes_effective_request_id_to_response_header() {
        let mut headers = HeaderMap::new();

        set_response_request_id_header(&mut headers, "trace-123", &config());

        assert_eq!(headers.get("x-request-id").unwrap(), "trace-123");
    }
}
