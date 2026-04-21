use bytes::Bytes;
use reqwest::StatusCode;
use tracing::warn;

use crate::domain::ProviderAccount;
use crate::gateway::vendor;

#[derive(Debug, Clone)]
pub struct UpstreamResponse {
    pub status: StatusCode,
    pub body: Bytes,
    pub content_type: Option<String>,
}

#[derive(Debug)]
pub enum UpstreamError {
    MissingBaseUrl {
        account_id: String,
        vendor: String,
    },
    MissingAuthHeader {
        vendor: String,
    },
    UpstreamStatus {
        status: StatusCode,
        body: Bytes,
        content_type: Option<String>,
    },
    Timeout {
        message: String,
    },
    Transport {
        message: String,
    },
    ReadBody {
        message: String,
    },
}

pub async fn proxy_chat_completion(
    client: &reqwest::Client,
    account: &ProviderAccount,
    raw_body: Bytes,
) -> Result<UpstreamResponse, UpstreamError> {
    let base =
        vendor::effective_base_url(account).ok_or_else(|| UpstreamError::MissingBaseUrl {
            account_id: account.id.to_string(),
            vendor: format!("{:?}", account.vendor),
        })?;
    let url = format!("{base}/v1/chat/completions");

    let mut request = client
        .post(&url)
        .header("content-type", "application/json")
        .body(raw_body);

    if let Some(api_key) = account.api_key.as_deref() {
        let (header_name, header_value) = vendor::auth_header(&account.vendor, api_key)
            .ok_or_else(|| UpstreamError::MissingAuthHeader {
                vendor: format!("{:?}", account.vendor),
            })?;
        request = request.header(header_name, &header_value);
    } else {
        warn!(
            account_id = %account.id,
            vendor = ?account.vendor,
            "proxying upstream request without API credentials"
        );
    }

    let response = request.send().await.map_err(|error| {
        if error.is_timeout() {
            UpstreamError::Timeout {
                message: format!("upstream request to {url} timed out: {error}"),
            }
        } else {
            UpstreamError::Transport {
                message: format!("upstream request to {url} failed: {error}"),
            }
        }
    })?;

    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let body = response
        .bytes()
        .await
        .map_err(|error| UpstreamError::ReadBody {
            message: format!("failed to read upstream response body: {error}"),
        })?;

    if !status.is_success() {
        return Err(UpstreamError::UpstreamStatus {
            status,
            body,
            content_type,
        });
    }

    Ok(UpstreamResponse {
        status,
        body,
        content_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use axum::body::Bytes as AxumBytes;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;
    use axum::{Json, Router};
    use bytes::Bytes;
    use serde_json::{json, Value};
    use tokio::net::TcpListener;

    use crate::domain::{AccountId, ProviderAccount, ProviderVendor};

    #[derive(Clone, Debug)]
    struct CapturedRequest {
        headers: HeaderMap,
        body: Value,
    }

    type MockState = (StatusCode, Value, Arc<Mutex<Vec<CapturedRequest>>>);

    fn make_account(vendor: ProviderVendor) -> ProviderAccount {
        ProviderAccount {
            id: AccountId::generate(),
            vendor,
            display_name: "test-account".to_string(),
            api_base_override: None,
            api_key: Some("sk-test".to_string()),
            enabled: true,
            weight: 1,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        }
    }

    async fn mock_upstream(
        status: StatusCode,
        body: Value,
        captured: Arc<Mutex<Vec<CapturedRequest>>>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        async fn handler(
            State((status, body, captured)): State<MockState>,
            headers: HeaderMap,
            raw_body: AxumBytes,
        ) -> (StatusCode, Json<Value>) {
            let parsed: Value = serde_json::from_slice(&raw_body).unwrap();
            captured.lock().unwrap().push(CapturedRequest {
                headers,
                body: parsed,
            });
            (status, Json(body))
        }

        let app = Router::new()
            .route("/v1/chat/completions", post(handler))
            .with_state((status, body, captured));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (format!("http://{addr}"), handle)
    }

    #[tokio::test]
    async fn proxy_chat_completion_happy_path_forwards_body_and_bearer_auth() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (base_url, _handle) = mock_upstream(
            StatusCode::OK,
            json!({"id":"chatcmpl-123","object":"chat.completion"}),
            captured.clone(),
        )
        .await;

        let mut account = make_account(ProviderVendor::OpenAi);
        account.api_base_override = Some(base_url);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let body = Bytes::from_static(
            br#"{"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]}"#,
        );

        let response = proxy_chat_completion(&client, &account, body.clone())
            .await
            .unwrap();

        assert_eq!(response.status, reqwest::StatusCode::OK);
        assert_eq!(
            response.body,
            Bytes::from_static(br#"{"id":"chatcmpl-123","object":"chat.completion"}"#)
        );

        let requests = captured.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].body,
            json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]})
        );
        assert_eq!(
            requests[0]
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer sk-test")
        );
    }

    #[tokio::test]
    async fn proxy_chat_completion_uses_override_base_url_and_anthropic_auth() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (base_url, _handle) =
            mock_upstream(StatusCode::OK, json!({"ok":true}), captured.clone()).await;

        let mut account = make_account(ProviderVendor::Anthropic);
        account.api_base_override = Some(format!("{base_url}/"));
        account.api_key = Some("sk-ant-123".to_string());

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let body = Bytes::from_static(
            br#"{"model":"claude-proxy","messages":[{"role":"user","content":"Hi"}]}"#,
        );

        let response = proxy_chat_completion(&client, &account, body)
            .await
            .unwrap();
        assert_eq!(response.status, reqwest::StatusCode::OK);

        let requests = captured.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0]
                .headers
                .get("x-api-key")
                .and_then(|value| value.to_str().ok()),
            Some("sk-ant-123")
        );
    }

    #[tokio::test]
    async fn proxy_chat_completion_without_api_key_still_forwards_request() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (base_url, _handle) =
            mock_upstream(StatusCode::OK, json!({"ok":true}), captured.clone()).await;

        let mut account = make_account(ProviderVendor::OpenAi);
        account.api_base_override = Some(base_url);
        account.api_key = None;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();

        let response = proxy_chat_completion(&client, &account, Bytes::from_static(br#"{}"#))
            .await
            .unwrap();

        assert_eq!(response.status, reqwest::StatusCode::OK);

        let requests = captured.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].headers.get("authorization").is_none());
        assert!(requests[0].headers.get("x-api-key").is_none());
    }

    #[tokio::test]
    async fn proxy_chat_completion_returns_local_error_for_missing_base_url() {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let account = make_account(ProviderVendor::Other("mistral".to_string()));

        let error = proxy_chat_completion(&client, &account, Bytes::from_static(br#"{}"#))
            .await
            .unwrap_err();

        assert!(matches!(error, UpstreamError::MissingBaseUrl { .. }));
    }

    #[tokio::test]
    async fn proxy_chat_completion_maps_upstream_non_success_status() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (base_url, _handle) = mock_upstream(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error":{"message":"boom"}}),
            captured,
        )
        .await;

        let mut account = make_account(ProviderVendor::OpenAi);
        account.api_base_override = Some(base_url);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();

        let error = proxy_chat_completion(&client, &account, Bytes::from_static(br#"{}"#))
            .await
            .unwrap_err();

        match error {
            UpstreamError::UpstreamStatus { status, body, .. } => {
                assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(body, Bytes::from_static(br#"{"error":{"message":"boom"}}"#));
            }
            other => panic!("expected UpstreamStatus error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn proxy_chat_completion_maps_transport_failures() {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(200))
            .build()
            .unwrap();
        let mut account = make_account(ProviderVendor::OpenAi);
        account.api_base_override = Some("http://127.0.0.1:9".to_string());

        let error = proxy_chat_completion(&client, &account, Bytes::from_static(br#"{}"#))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            UpstreamError::Transport { .. } | UpstreamError::Timeout { .. }
        ));
    }
}
