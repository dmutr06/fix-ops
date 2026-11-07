//! Router and shared state.

mod error;
mod json;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use fixops_app::App;
use fixops_infra::PgStore;
use tower_http::trace::TraceLayer;

pub use error::ApiError;
pub use json::Json;

/// State shared by all handlers.
#[derive(Debug, Clone)]
pub struct AppState {
    pub app: Arc<App<PgStore>>,
}

impl AppState {
    pub fn new(store: PgStore) -> Self {
        Self {
            app: Arc::new(App::new(store)),
        }
    }
}

/// Builds the full HTTP router.
pub fn router(state: AppState) -> Router {
    let api = Router::new();

    Router::new()
        .route("/health", get(health))
        .nest("/api/v1", api)
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn not_found() -> ApiError {
    fixops_app::AppError::not_found().into()
}
