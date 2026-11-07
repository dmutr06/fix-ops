//! The audit log: the shared writer every store uses inside its transactions, and the
//! [`AuditStore`] reader.

use fixops_app::StoreError;
use fixops_app::audit::{AuditEntry, AuditFilter, AuditStore};
use fixops_domain::audit::{FieldChange, RecordType};
use fixops_domain::id::StaffId;
use sqlx::{FromRow, PgConnection};
use time::OffsetDateTime;

use crate::PgStore;
use crate::error::{decode_error, store_error};

/// Writes one audit row per change, attributed to `actor`. Call it on the transaction that
/// writes the record itself, so the change and its audit entries commit or fail together.
pub(crate) async fn insert_audit(
    conn: &mut PgConnection,
    actor: StaffId,
    record_type: RecordType,
    record_id: i64,
    changes: &[FieldChange],
) -> Result<(), sqlx::Error> {
    let fields: Vec<&str> = changes.iter().map(|c| c.field).collect();
    let previous: Vec<Option<&str>> = changes.iter().map(|c| c.previous.as_deref()).collect();
    let new: Vec<Option<&str>> = changes.iter().map(|c| c.new.as_deref()).collect();

    sqlx::query(
        "INSERT INTO audit_log (actor_id, record_type, record_id, field, previous_value, new_value)
         SELECT $1, $2, $3, c.field, c.previous_value, c.new_value
         FROM UNNEST($4::text[], $5::text[], $6::text[])
             WITH ORDINALITY AS c(field, previous_value, new_value, n)
         ORDER BY c.n",
    )
    .bind(actor.0)
    .bind(record_type.as_str())
    .bind(record_id)
    .bind(fields)
    .bind(previous)
    .bind(new)
    .execute(conn)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct AuditRow {
    id: i64,
    actor_id: i64,
    recorded_at: OffsetDateTime,
    record_type: String,
    record_id: i64,
    field: String,
    previous_value: Option<String>,
    new_value: Option<String>,
}

impl TryFrom<AuditRow> for AuditEntry {
    type Error = sqlx::Error;

    fn try_from(row: AuditRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            actor_id: StaffId(row.actor_id),
            recorded_at: row.recorded_at,
            record_type: row.record_type.parse().map_err(decode_error)?,
            record_id: row.record_id,
            field: row.field,
            previous_value: row.previous_value,
            new_value: row.new_value,
        })
    }
}

impl AuditStore for PgStore {
    async fn list_audit_entries(&self, filter: AuditFilter) -> Result<Vec<AuditEntry>, StoreError> {
        sqlx::query_as::<_, AuditRow>(
            "SELECT id, actor_id, recorded_at, record_type, record_id, field,
                    previous_value, new_value
             FROM audit_log
             WHERE ($1::text IS NULL OR record_type = $1)
               AND ($2::bigint IS NULL OR record_id = $2)
             ORDER BY recorded_at, id",
        )
        .bind(filter.record_type.map(RecordType::as_str))
        .bind(filter.record_id)
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?
        .into_iter()
        .map(|row| AuditEntry::try_from(row).map_err(store_error))
        .collect()
    }
}
