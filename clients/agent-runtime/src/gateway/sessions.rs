use crate::gateway::{compute_token_hash, AppState};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};

#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct SessionListParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Minimal session view for end-users — no metadata or memory content.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UserSessionView {
    pub id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub message_count: u32,
    pub last_activity: String,
}

/// GET /session/list — end-user session list scoped by bearer token.
///
/// Returns only sessions associated with the caller's token hash.
/// MUST NOT include metadata or memory content.
pub async fn handle_session_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<SessionListParams>,
) -> impl IntoResponse {
    // Require bearer token auth (any authenticated user, not admin-only)
    let token = match crate::gateway::utils::extract_bearer_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Unauthorized — send Authorization: Bearer <token>"
                })),
            );
        }
    };

    if state.pairing.require_pairing() && !state.pairing.is_authenticated(&token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized — invalid bearer token"
            })),
        );
    }

    if !state.pairing.require_pairing()
        && !state.config.lock().gateway.allow_unpaired_session_scopes
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized — unpaired session scopes are disabled"
            })),
        );
    }

    let token_hash = compute_token_hash(&token);
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    match state
        .mem
        .list_sessions_for_token(&token_hash, limit, offset)
        .await
    {
        Ok((sessions, total)) => {
            let views: Vec<UserSessionView> = sessions
                .into_iter()
                .map(|s| UserSessionView {
                    id: s.id,
                    started_at: s.started_at,
                    ended_at: s.ended_at,
                    message_count: s.message_count,
                    last_activity: s.last_activity,
                })
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "sessions": views,
                    "total": total,
                })),
            )
        }
        Err(e) => {
            tracing::error!("session list failed: {e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list sessions"})),
            )
        }
    }
}
