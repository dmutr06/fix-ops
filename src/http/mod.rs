//! Router and shared state.

mod audit;
mod auth;
mod clients;
mod devices;
mod error;
mod json;
mod orders;
mod params;
mod sessions;
mod staff;
mod trace;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use fixops_app::App;
use fixops_infra::PgStore;

pub use auth::SignedIn;
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
    let api = Router::new()
        .merge(sessions::routes())
        .merge(staff::routes())
        .merge(clients::routes())
        .merge(devices::routes())
        .merge(orders::routes())
        .merge(audit::routes());

    let router = Router::new()
        .route("/health", get(health))
        .nest("/api/v1", api)
        .fallback(not_found);
    trace::layer(router).with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn not_found() -> ApiError {
    fixops_app::AppError::not_found().into()
}
