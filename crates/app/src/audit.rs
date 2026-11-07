//! Reading the audit log (REQ-F-018, REQ-NF-003). Entries are written by each store together
//! with the change they describe; this module only reads them.

use fixops_domain::audit::RecordType;
use fixops_domain::id::StaffId;
use fixops_domain::staff::{Action, Actor};
use time::OffsetDateTime;

use crate::{App, AppResult, StoreError};

/// One audited field change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    pub id: i64,
    pub actor_id: StaffId,
    pub recorded_at: OffsetDateTime,
    pub record_type: RecordType,
    pub record_id: i64,
    pub field: String,
    pub previous_value: Option<String>,
    pub new_value: Option<String>,
}

/// Which entries to return. Unset fields match everything.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AuditFilter {
    pub record_type: Option<RecordType>,
    pub record_id: Option<i64>,
}

/// Port: read access to the audit log.
pub trait AuditStore: Send + Sync {
    /// Matching entries in the order they were recorded.
    fn list_audit_entries(
        &self,
        filter: AuditFilter,
    ) -> impl Future<Output = Result<Vec<AuditEntry>, StoreError>> + Send;
}

impl<S: AuditStore> App<S> {
    /// Audit entries matching `filter`, oldest first.
    pub async fn list_audit_logs(
        &self,
        actor: Actor,
        filter: AuditFilter,
    ) -> AppResult<Vec<AuditEntry>> {
        actor.require(Action::ListAuditLogs)?;
        Ok(self.store.list_audit_entries(filter).await?)
    }
}
