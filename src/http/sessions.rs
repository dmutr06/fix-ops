//! `/sessions`: staff sign-in, the current session and sign-out (REQ-F-001, REQ-F-019,
//! REQ-F-022). Sign-in is the only route without a session.

use axum::Router;
use axum::extract::State;
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use fixops_app::AppError;
use fixops_app::staff::SESSION_LIFETIME;
use fixops_domain::staff::Action;
use serde::{Deserialize, Serialize};

use super::auth::{SESSION_COOKIE, SignedIn, session_token};
use super::staff::StaffAccountDto;
use super::{ApiError, AppState, Json};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sessions", post(create))
        .route("/sessions/current", get(current).delete(sign_out))
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

/// `Set-Cookie` for the browser session; scripts cannot read it (ADD-004).
fn session_cookie(token: &str, max_age: i64) -> String {
    format!(
        "{SESSION_COOKIE}={token}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={max_age}"
    )
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<SessionCreateRequest>,
) -> Result<
    (
        StatusCode,
        [(axum::http::HeaderName, String); 1],
        Json<SessionResponse>,
    ),
    ApiError,
> {
    let session = state.app.sign_in(&body.username, &body.password).await?;
    let token = session.token.into_string();
    let cookie = session_cookie(&token, SESSION_LIFETIME.whole_seconds());
    let response = SessionResponse {
        token,
        staff: session.account.into(),
    };
    Ok((StatusCode::CREATED, [(SET_COOKIE, cookie)], Json(response)))
}

async fn current(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
) -> Result<Json<StaffAccountDto>, ApiError> {
    let account = state.app.current_staff(actor).await?;
    Ok(Json(account.into()))
}

async fn sign_out(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    headers: HeaderMap,
) -> Result<(StatusCode, [(axum::http::HeaderName, String); 1]), ApiError> {
    state.app.authorize(actor, Action::DeleteSession)?;
    let token = session_token(&headers).ok_or_else(AppError::unauthenticated)?;
    state.app.sign_out(actor, token).await?;
    Ok((
        StatusCode::NO_CONTENT,
        [(SET_COOKIE, session_cookie("", 0))],
    ))
}
