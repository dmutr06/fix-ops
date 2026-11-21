//! The staff web app (REQ-F-021): files of `web/dist`, embedded in the binary in release builds
//! and read from disk in debug builds. Paths outside `/api` that are not files get
//! `index.html`, so the app's own routes survive a reload.

use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{Method, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

use super::ApiError;

#[derive(Embed)]
#[folder = "web/dist"]
#[allow_missing = true]
struct Assets;

/// Answers every request no route matched.
pub async fn fallback(method: Method, uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if method != Method::GET || path == "api" || path.starts_with("api/") {
        return not_found();
    }
    let asset = (!path.is_empty())
        .then(|| Assets::get(path).map(|file| (path, file)))
        .flatten()
        .or_else(|| Assets::get("index.html").map(|file| ("index.html", file)));
    let Some((name, file)) = asset else {
        return not_found();
    };
    // Built assets have content hashes in their names; the page itself must not be cached.
    let cache = if name.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        [
            (CONTENT_TYPE, file.metadata.mimetype().to_owned()),
            (CACHE_CONTROL, cache.to_owned()),
        ],
        file.data,
    )
        .into_response()
}

fn not_found() -> Response {
    ApiError::from(fixops_app::AppError::not_found()).into_response()
}
