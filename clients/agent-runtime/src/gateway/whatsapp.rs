use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use crate::gateway::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct WhatsAppVerifyQuery {
    #[serde(rename = "hub.mode")]
    pub mode: String,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: String,
    #[serde(rename = "hub.challenge")]
    pub challenge: String,
}

pub async fn handle_whatsapp_verify(
    State(state): State<AppState>,
    Query(query): Query<WhatsAppVerifyQuery>,
) -> impl IntoResponse {
    let whatsapp = match state.whatsapp {
        Some(ref wa) => wa,
        None => {
            return (
                StatusCode::NOT_IMPLEMENTED,
                "WhatsApp channel not configured",
            )
                .into_response();
        }
    };

    if query.mode == "subscribe" && query.verify_token == whatsapp.verify_token() {
        tracing::info!("WhatsApp webhook verified successfully");
        query.challenge.into_response()
    } else {
        tracing::warn!("WhatsApp webhook verification failed: invalid token");
        (StatusCode::FORBIDDEN, "Invalid verify token").into_response()
    }
}

pub async fn handle_whatsapp_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    let whatsapp = match state.whatsapp {
        Some(ref wa) => wa,
        None => {
            return (
                StatusCode::NOT_IMPLEMENTED,
                Json(serde_json::json!({
                    "error": "WhatsApp channel not configured"
                })),
            )
                .into_response();
        }
    };

    // Fail closed: require app_secret to be configured
    let app_secret = match state.whatsapp_app_secret.as_ref() {
        Some(secret) => secret,
        None => {
            tracing::error!("WhatsApp webhook received but whatsapp_app_secret is not configured");
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": "WhatsApp webhook not configured securely"
                })),
            )
                .into_response();
        }
    };

    // Verify signature using constant-time comparison
    let signature = headers
        .get("X-Hub-Signature-256")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !verify_whatsapp_signature(app_secret, body.as_bytes(), signature) {
        tracing::warn!("WhatsApp webhook signature verification failed");
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Invalid signature"})),
        )
            .into_response();
    }

    let payload: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid JSON body"})),
            )
                .into_response();
        }
    };

    let messages = whatsapp.parse_webhook_payload(&payload);
    for msg in messages {
        state.observer.record_event(&crate::observability::ObserverEvent::ChannelMessage {
            channel: "whatsapp".into(),
            direction: "inbound".into(),
        });

        // WhatsApp messages are handled by the main provider
        match state.provider.chat_with_system(None, &msg.content, &state.model, state.temperature).await {
            Ok(response) => {
                state.observer.record_event(&crate::observability::ObserverEvent::TurnComplete);

                let reply = crate::channels::SendMessage::new(response, msg.reply_target);

                use crate::channels::Channel;
                if let Err(err) = whatsapp.send(&reply).await {
                    tracing::error!("Failed to send WhatsApp reply: {err:#}");
                }
            }
            Err(err) => {
                tracing::error!("WhatsApp AI provider error: {err:#}");
            }
        }
    }

    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}

/// Verify WhatsApp webhook signature using constant-time comparison.
///
/// The signature is expected in the format "sha256=<hex_signature>".
/// Uses Mac::verify_slice for constant-time verification to prevent timing attacks.
pub fn verify_whatsapp_signature(app_secret: &str, body: &[u8], signature: &str) -> bool {
    let signature = signature.strip_prefix("sha256=").unwrap_or(signature);

    // Decode hex signature to bytes
    let signature_bytes = match hex::decode(signature) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    // Create HMAC and compute MAC
    let mut mac = match Hmac::<Sha256>::new_from_slice(app_secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(body);

    // Use verify_slice for constant-time comparison
    mac.verify_slice(&signature_bytes).is_ok()
}
