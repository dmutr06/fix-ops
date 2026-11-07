//! `/staff`: staff account management by Administrator (REQ-F-019).

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, patch};
use fixops_app::{AppError, ErrorKind};
use fixops_domain::id::StaffId;
use fixops_domain::staff::{NewStaffAccount, Password, Role, StaffAccount, Username};
use serde::{Deserialize, Serialize};

use super::auth::SignedIn;
use super::params::PathId;
use super::{ApiError, AppState, Json};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/staff", get(list).post(create))
        .route("/staff/{staffId}", patch(update))
}

/// Contract `StaffAccount`. The password hash is never part of it.
#[derive(Debug, Serialize)]
pub struct StaffAccountDto {
    id: i64,
    username: String,
    role: &'static str,
    enabled: bool,
}

impl From<StaffAccount> for StaffAccountDto {
    fn from(account: StaffAccount) -> Self {
        Self {
            id: account.id.0,
            username: account.username.into_string(),
            role: account.role.as_str(),
            enabled: account.enabled,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StaffCreateRequest {
    username: String,
    password: String,
    role: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StaffUpdateRequest {
    role: Option<String>,
    enabled: Option<bool>,
}

fn parse_role(raw: &str) -> Result<Role, AppError> {
    raw.parse().map_err(|_| {
        AppError::validation(
            "role",
            "role must be Receptionist, Technician or Administrator",
        )
    })
}

async fn list(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
) -> Result<Json<Vec<StaffAccountDto>>, ApiError> {
    let accounts = state.app.list_staff(actor).await?;
    Ok(Json(accounts.into_iter().map(Into::into).collect()))
}

async fn create(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    Json(body): Json<StaffCreateRequest>,
) -> Result<(StatusCode, Json<StaffAccountDto>), ApiError> {
    let account = NewStaffAccount {
        username: Username::parse(&body.username)?,
        password: Password::parse(body.password)?,
        role: parse_role(&body.role)?,
    };
    let created = state.app.create_staff(actor, account).await?;
    Ok((StatusCode::CREATED, Json(created.into())))
}

async fn update(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    PathId(id): PathId,
    Json(body): Json<StaffUpdateRequest>,
) -> Result<Json<StaffAccountDto>, ApiError> {
    if body.role.is_none() && body.enabled.is_none() {
        return Err(AppError::new(
            ErrorKind::Validation,
            "at least one of role or enabled is required",
        )
        .into());
    }
    let role = body.role.as_deref().map(parse_role).transpose()?;
    let updated = state
        .app
        .update_staff(actor, StaffId(id), role, body.enabled)
        .await?;
    Ok(Json(updated.into()))
}
