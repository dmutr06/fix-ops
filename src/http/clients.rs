//! `/clients`: client records, search and loyalty tiers (REQ-F-003, REQ-F-017).

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, put};
use fixops_app::{AppError, ErrorKind};
use fixops_domain::client::{
    Client, ClientName, ClientPatch, Email, LoyaltyTier, NewClient, Phone,
};
use fixops_domain::id::ClientId;
use fixops_domain::staff::Action;
use serde::{Deserialize, Serialize};

use super::auth::SignedIn;
use super::json::{self, Json};
use super::params::{PathId, Query, non_empty};
use super::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/clients", get(list).post(create))
        .route("/clients/{clientId}", get(show).patch(update))
        .route("/clients/{clientId}/loyalty-tier", put(set_loyalty_tier))
}

/// Contract `Client`. `email` is `null` when the client has none.
#[derive(Debug, Serialize)]
struct ClientDto {
    id: i64,
    name: String,
    phone: String,
    email: Option<String>,
    loyalty_tier: &'static str,
}

impl From<Client> for ClientDto {
    fn from(client: Client) -> Self {
        Self {
            id: client.id.0,
            name: client.name.as_str().to_owned(),
            phone: client.phone.as_str().to_owned(),
            email: client.email.as_ref().map(|email| email.as_str().to_owned()),
            loyalty_tier: client.loyalty_tier.as_str(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientQuery {
    q: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientCreateRequest {
    name: String,
    phone: String,
    #[serde(default)]
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientUpdateRequest {
    #[serde(default, deserialize_with = "json::not_null")]
    name: Option<String>,
    #[serde(default, deserialize_with = "json::not_null")]
    phone: Option<String>,
    /// Absent keeps the email; `null` clears it (the contract marks it `nullable`).
    #[expect(
        clippy::option_option,
        reason = "absent, null and a value are three different requests"
    )]
    #[serde(default, deserialize_with = "json::not_null")]
    email: Option<Option<String>>,
}

impl ClientUpdateRequest {
    fn into_patch(self) -> Result<ClientPatch, AppError> {
        if self.name.is_none() && self.phone.is_none() && self.email.is_none() {
            return Err(AppError::new(
                ErrorKind::Validation,
                "at least one of name, phone or email is required",
            ));
        }
        Ok(ClientPatch {
            name: self.name.as_deref().map(ClientName::parse).transpose()?,
            phone: self.phone.as_deref().map(Phone::parse).transpose()?,
            email: self
                .email
                .map(|email| parse_email(email.as_deref()))
                .transpose()?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LoyaltyTierUpdateRequest {
    loyalty_tier: String,
}

fn parse_email(raw: Option<&str>) -> Result<Option<Email>, AppError> {
    Ok(raw.map(Email::parse).transpose()?)
}

async fn list(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    query: Result<Query<ClientQuery>, ApiError>,
) -> Result<Json<Vec<ClientDto>>, ApiError> {
    state.app.authorize(actor, Action::ListClients)?;
    let Query(query) = query?;
    let text = non_empty("q", query.q)?;
    let clients = state.app.list_clients(actor, text.as_deref()).await?;
    Ok(Json(clients.into_iter().map(Into::into).collect()))
}

async fn create(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    body: Result<Json<ClientCreateRequest>, ApiError>,
) -> Result<(StatusCode, Json<ClientDto>), ApiError> {
    state.app.authorize(actor, Action::CreateClient)?;
    let Json(body) = body?;
    let client = NewClient {
        name: ClientName::parse(&body.name)?,
        phone: Phone::parse(&body.phone)?,
        email: parse_email(body.email.as_deref())?,
    };
    let created = state.app.create_client(actor, client).await?;
    Ok((StatusCode::CREATED, Json(created.into())))
}

async fn show(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    id: Result<PathId, ApiError>,
) -> Result<Json<ClientDto>, ApiError> {
    state.app.authorize(actor, Action::GetClient)?;
    let PathId(id) = id?;
    let client = state.app.get_client(actor, ClientId(id)).await?;
    Ok(Json(client.into()))
}

async fn update(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    id: Result<PathId, ApiError>,
    body: Result<Json<ClientUpdateRequest>, ApiError>,
) -> Result<Json<ClientDto>, ApiError> {
    state.app.authorize(actor, Action::UpdateClient)?;
    let PathId(id) = id?;
    let Json(body) = body?;
    let updated = state
        .app
        .update_client(actor, ClientId(id), body.into_patch()?)
        .await?;
    Ok(Json(updated.into()))
}

async fn set_loyalty_tier(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    id: Result<PathId, ApiError>,
    body: Result<Json<LoyaltyTierUpdateRequest>, ApiError>,
) -> Result<Json<ClientDto>, ApiError> {
    state.app.authorize(actor, Action::SetClientLoyaltyTier)?;
    let PathId(id) = id?;
    let Json(body) = body?;
    let tier = LoyaltyTier::parse(&body.loyalty_tier)?;
    let updated = state
        .app
        .set_loyalty_tier(actor, ClientId(id), tier)
        .await?;
    Ok(Json(updated.into()))
}
