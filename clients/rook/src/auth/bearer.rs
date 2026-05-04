use axum::http::{header::AUTHORIZATION, HeaderMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BearerExtractionError {
    Missing,
    InvalidScheme,
    EmptyToken,
    Malformed,
}

pub fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, BearerExtractionError> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let value = values.next().ok_or(BearerExtractionError::Missing)?;
    if values.next().is_some() {
        return Err(BearerExtractionError::Malformed);
    }

    let raw = value
        .to_str()
        .map_err(|_| BearerExtractionError::Malformed)?;
    let mut parts = raw.split_whitespace();
    let scheme = parts.next().ok_or(BearerExtractionError::Malformed)?;
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return Err(BearerExtractionError::InvalidScheme);
    }

    let token = parts.next().ok_or(BearerExtractionError::EmptyToken)?;
    if token.trim().is_empty() {
        return Err(BearerExtractionError::EmptyToken);
    }
    if parts.next().is_some() {
        return Err(BearerExtractionError::Malformed);
    }

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::extract_bearer_token;
    use axum::http::{header::AUTHORIZATION, HeaderMap, HeaderValue};

    #[test]
    fn extract_bearer_token_rejects_missing_header() {
        let headers = HeaderMap::new();

        assert!(extract_bearer_token(&headers).is_err());
    }

    #[test]
    fn extract_bearer_token_rejects_non_bearer_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc123"));

        assert!(extract_bearer_token(&headers).is_err());
    }

    #[test]
    fn extract_bearer_token_rejects_empty_token() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer   "));

        assert!(extract_bearer_token(&headers).is_err());
    }

    #[test]
    fn extract_bearer_token_accepts_valid_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer rook-inbound-secret"),
        );

        assert_eq!(
            extract_bearer_token(&headers).unwrap(),
            "rook-inbound-secret"
        );
    }

    #[test]
    fn extract_bearer_token_accepts_case_insensitive_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("bearer rook-inbound-secret"),
        );

        assert_eq!(
            extract_bearer_token(&headers).unwrap(),
            "rook-inbound-secret"
        );
    }

    #[test]
    fn extract_bearer_token_trims_surrounding_whitespace() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer   rook-inbound-secret   "),
        );

        assert_eq!(
            extract_bearer_token(&headers).unwrap(),
            "rook-inbound-secret"
        );
    }

    #[test]
    fn extract_bearer_token_rejects_ambiguous_or_malformed_values() {
        let mut duplicate_headers = HeaderMap::new();
        duplicate_headers.append(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer rook-inbound-secret"),
        );
        duplicate_headers.append(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer second-token"),
        );
        assert!(extract_bearer_token(&duplicate_headers).is_err());

        let mut malformed_headers = HeaderMap::new();
        malformed_headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer rook-inbound-secret extra"),
        );
        assert!(extract_bearer_token(&malformed_headers).is_err());
    }
}
