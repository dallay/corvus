//! Dashboard — embedded admin UI served from binary assets.
//!
//! Static assets under `../assets/` are embedded at compile time via
//! [`rust_embed::RustEmbed`]. The [`router`] function returns an
//! [`axum::Router`] that serves those assets over HTTP:
//!
//! - `GET /`             → `index.html`
//! - `GET /assets/*path` → any other embedded file
//! - Everything else     → 404

use axum::{
    body::Body,
    http::{header, Response, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use rust_embed::RustEmbed;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Embedded asset bundle
// ---------------------------------------------------------------------------

/// All files under `assets/` embedded into the binary at compile time.
#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct DashboardAssets;

static ASSETS_READY_OVERRIDE: OnceLock<Mutex<Option<bool>>> = OnceLock::new();

fn lock_assets_ready_override() -> std::sync::MutexGuard<'static, Option<bool>> {
    ASSETS_READY_OVERRIDE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

pub fn assets_ready() -> bool {
    if let Some(override_value) = lock_assets_ready_override().as_ref().copied() {
        return override_value;
    }

    DashboardAssets::get("index.html").is_some()
}

#[doc(hidden)]
pub struct AssetsReadyOverrideGuard;

impl AssetsReadyOverrideGuard {
    pub fn new(value: bool) -> Self {
        *lock_assets_ready_override() = Some(value);
        Self
    }
}

impl Drop for AssetsReadyOverrideGuard {
    fn drop(&mut self) {
        *lock_assets_ready_override() = None;
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Serve the root `index.html`.
async fn serve_index() -> impl IntoResponse {
    serve_asset("index.html").await
}

/// Serve an arbitrary embedded asset by path.
async fn serve_embedded(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl IntoResponse {
    serve_asset(&path).await
}

/// Look up `name` in [`DashboardAssets`] and build a `Response`, or 404.
async fn serve_asset(name: &str) -> Response<Body> {
    match DashboardAssets::get(name) {
        Some(content) => {
            let mime = mime_guess::from_path(name).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data.into_owned()))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .expect("infallible 500 response")
                })
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .expect("infallible 404 response"),
    }
}

// ---------------------------------------------------------------------------
// Public router
// ---------------------------------------------------------------------------

/// Returns an [`axum::Router`] that serves embedded dashboard assets.
///
/// Mount this at the root of the combined server router.
pub fn router() -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/assets/{*path}", get(serve_embedded))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_html_is_embedded() {
        let file = DashboardAssets::get("index.html");
        assert!(
            file.is_some(),
            "index.html must be embedded in DashboardAssets"
        );
        let asset = file.unwrap();
        let content = std::str::from_utf8(&asset.data).expect("valid UTF-8");
        assert!(
            content.contains("Corvus Rook"),
            "index.html should contain 'Corvus Rook'"
        );
    }

    #[test]
    fn missing_file_returns_none() {
        assert!(DashboardAssets::get("nonexistent.js").is_none());
    }

    #[test]
    fn dashboard_router_is_constructible() {
        // Just verifying the router builds without panicking.
        let _r = router();
    }
}
