//! `/sessions`: staff sign-in (REQ-F-001, REQ-F-019). The only route without a session.

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use serde::{Deserialize, Serialize};

use super::staff::StaffAccountDto;
use super::{ApiError, AppState, Json};

pub fn routes() -> Router<AppState> {
    Router::new().route("/sessions", post(create))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionCreateRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct SessionResponse {
    token: String,
    staff: StaffAccountDto,
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<SessionCreateRequest>,
) -> Result<(StatusCode, Json<SessionResponse>), ApiError> {
    let session = state.app.sign_in(&body.username, &body.password).await?;
    let response = SessionResponse {
        token: session.token.into_string(),
        staff: session.account.into(),
    };
    Ok((StatusCode::CREATED, Json(response)))
}
