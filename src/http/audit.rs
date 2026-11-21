//! `/audit-logs`: the change history, readable by every signed-in role (REQ-NF-003, ADD-002).

use axum::Router;
use axum::extract::State;
use axum::routing::get;
use fixops_app::audit::{AuditEntry, AuditFilter};
use fixops_app::{AppError, Page};
use fixops_domain::audit::RecordType;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::auth::SignedIn;
use super::params::{Listed, Query, page, positive_id};
use super::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/audit-logs", get(list))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditQuery {
    record_type: Option<String>,
    record_id: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
}

impl AuditQuery {
    fn into_filter(self) -> Result<(AuditFilter, Page), AppError> {
        let record_type = self
            .record_type
            .map(|raw| {
                raw.parse::<RecordType>().map_err(|_| {
                    AppError::validation(
                        "record_type",
                        "record_type must be StaffAccount, Client, Device or RepairOrder",
                    )
                })
            })
            .transpose()?;
        let filter = AuditFilter {
            record_type,
            record_id: positive_id("record_id", self.record_id)?,
        };
        Ok((filter, page(self.limit, self.offset)?))
    }
}

/// Contract `AuditEntry`.
#[derive(Debug, Serialize)]
struct AuditEntryDto {
    id: i64,
    actor_id: i64,
    #[serde(with = "time::serde::rfc3339")]
    recorded_at: OffsetDateTime,
    record_type: &'static str,
    record_id: i64,
    field: String,
    previous_value: Option<String>,
    new_value: Option<String>,
}

impl From<AuditEntry> for AuditEntryDto {
    fn from(entry: AuditEntry) -> Self {
        Self {
            id: entry.id,
            actor_id: entry.actor_id.0,
            recorded_at: entry.recorded_at,
            record_type: entry.record_type.as_str(),
            record_id: entry.record_id,
            field: entry.field,
            previous_value: entry.previous_value,
            new_value: entry.new_value,
        }
    }
}

async fn list(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    Query(query): Query<AuditQuery>,
) -> Result<Listed<AuditEntryDto>, ApiError> {
    let (filter, page) = query.into_filter()?;
    let entries = state.app.list_audit_logs(actor, filter, page).await?;
    Ok(Listed(entries.map(Into::into)))
}
