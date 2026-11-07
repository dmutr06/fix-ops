//! [`OrderStore`]: repair orders.

use fixops_app::StoreError;
use fixops_app::orders::{OrderFilter, OrderStore};
use fixops_domain::audit::{FieldChange, RecordType};
use fixops_domain::id::{ClientId, DeviceId, OrderId, StaffId};
use fixops_domain::money::Cents;
use fixops_domain::order::{
    ClientDecision, Decision, DiagnosticFindings, Estimate, NewRepairOrder, OrderStatus, Payment,
    RepairOrder, TicketNumber,
};
use fixops_domain::pricing::Settlement;
use fixops_domain::staff::StaffAccount;
use sqlx::FromRow;
use time::OffsetDateTime;

use crate::PgStore;
use crate::audit::insert_audit;
use crate::error::{decode_error, store_error};

/// The columns of [`OrderRow`], as a literal so queries stay static strings.
macro_rules! columns {
    () => {
        "id, ticket_number, client_id, device_id, assigned_technician_id, status,
         diagnostic_findings, estimate_parts_cents, estimate_labor_cents, client_decision,
         client_decided_at, used_parts_cents, completed_labor_cents, repair_checked,
         payment_cents, paid_at, picked_up_at, labor_discount_cents, settlement_cents, cancelled_from, version"
    };
}

#[derive(FromRow)]
struct OrderRow {
    id: i64,
    ticket_number: String,
    client_id: i64,
    device_id: i64,
    assigned_technician_id: i64,
    status: String,
    diagnostic_findings: Option<String>,
    estimate_parts_cents: Option<i64>,
    estimate_labor_cents: Option<i64>,
    client_decision: Option<String>,
    client_decided_at: Option<OffsetDateTime>,
    used_parts_cents: i64,
    completed_labor_cents: i64,
    repair_checked: bool,
    payment_cents: Option<i64>,
    paid_at: Option<OffsetDateTime>,
    picked_up_at: Option<OffsetDateTime>,
    labor_discount_cents: Option<i64>,
    settlement_cents: Option<i64>,
    cancelled_from: Option<String>,
    version: i64,
}

fn cents(field: &'static str, value: i64) -> Result<Cents, sqlx::Error> {
    Cents::new(field, value).map_err(decode_error)
}

impl TryFrom<OrderRow> for RepairOrder {
    type Error = sqlx::Error;

    fn try_from(row: OrderRow) -> Result<Self, Self::Error> {
        let estimate = match (row.estimate_parts_cents, row.estimate_labor_cents) {
            (Some(parts), Some(labor)) => Some(Estimate {
                parts: cents("parts_cents", parts)?,
                labor: cents("labor_cents", labor)?,
            }),
            _ => None,
        };
        let client_decision = match (row.client_decision, row.client_decided_at) {
            (Some(decision), Some(decided_at)) => Some(ClientDecision {
                decision: Decision::parse(&decision).map_err(decode_error)?,
                decided_at,
            }),
            _ => None,
        };
        let payment = match (row.payment_cents, row.paid_at) {
            (Some(amount), Some(paid_at)) => Some(Payment {
                amount: cents("amount_cents", amount)?,
                paid_at,
            }),
            _ => None,
        };
        let final_settlement = match (row.labor_discount_cents, row.settlement_cents) {
            (Some(discount), Some(total)) => Some(Settlement {
                labor_discount: cents("labor_discount_cents", discount)?,
                total: cents("settlement_cents", total)?,
            }),
            _ => None,
        };
        Ok(Self {
            id: OrderId(row.id),
            ticket_number: TicketNumber::parse(&row.ticket_number).map_err(decode_error)?,
            client_id: ClientId(row.client_id),
            device_id: DeviceId(row.device_id),
            assigned_technician_id: StaffId(row.assigned_technician_id),
            status: OrderStatus::parse(&row.status).map_err(decode_error)?,
            diagnostic_findings: row
                .diagnostic_findings
                .as_deref()
                .map(DiagnosticFindings::parse)
                .transpose()
                .map_err(decode_error)?,
            estimate,
            client_decision,
            used_parts: cents("used_parts_cents", row.used_parts_cents)?,
            completed_labor: cents("completed_labor_cents", row.completed_labor_cents)?,
            repair_checked: row.repair_checked,
            payment,
            picked_up_at: row.picked_up_at,
            final_settlement,
            cancelled_from: row
                .cancelled_from
                .as_deref()
                .map(OrderStatus::parse)
                .transpose()
                .map_err(decode_error)?,
            version: row.version,
        })
    }
}

impl OrderStore for PgStore {
    async fn create_order(
        &self,
        actor: StaffId,
        order: &NewRepairOrder,
        changes: &[FieldChange],
    ) -> Result<RepairOrder, StoreError> {
        let mut tx = self.pool().begin().await.map_err(store_error)?;
        let created: RepairOrder = sqlx::query_as::<_, OrderRow>(concat!(
            "INSERT INTO repair_orders
                 (ticket_number, client_id, device_id, assigned_technician_id, status)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING ",
            columns!()
        ))
        .bind(order.ticket_number.as_str())
        .bind(order.client_id.0)
        .bind(order.device_id.0)
        .bind(order.assigned_technician_id.0)
        .bind(NewRepairOrder::STATUS.as_str())
        .fetch_one(&mut *tx)
        .await
        .and_then(RepairOrder::try_from)
        .map_err(store_error)?;
        insert_audit(
            &mut tx,
            actor,
            RecordType::RepairOrder,
            created.id.0,
            changes,
        )
        .await
        .map_err(store_error)?;
        tx.commit().await.map_err(store_error)?;
        Ok(created)
    }

    async fn update_order(
        &self,
        actor: StaffId,
        order: &RepairOrder,
        changes: &[FieldChange],
    ) -> Result<(), StoreError> {
        let mut tx = self.pool().begin().await.map_err(store_error)?;
        save_order(&mut tx, actor, order, changes).await?;
        tx.commit().await.map_err(store_error)
    }

    async fn settle_order(
        &self,
        actor: StaffId,
        order: &RepairOrder,
        client_version: i64,
        changes: &[FieldChange],
    ) -> Result<(), StoreError> {
        let mut tx = self.pool().begin().await.map_err(store_error)?;
        // This lock conflicts with client UPDATE and stays held through the order/audit commit.
        // A version check without the lock would leave a tier-change race after the read.
        let version: Option<i64> =
            sqlx::query_scalar("SELECT version FROM clients WHERE id = $1 FOR UPDATE")
                .bind(order.client_id.0)
                .fetch_optional(&mut *tx)
                .await
                .map_err(store_error)?;
        if version != Some(client_version) {
            return Err(StoreError::Stale);
        }
        save_order(&mut tx, actor, order, changes).await?;
        tx.commit().await.map_err(store_error)
    }

    async fn correct_order(
        &self,
        actor: StaffId,
        order: &RepairOrder,
        client_version: i64,
        assignee: &StaffAccount,
        changes: &[FieldChange],
    ) -> Result<(), StoreError> {
        let mut tx = self.pool().begin().await.map_err(store_error)?;
        let version: Option<i64> =
            sqlx::query_scalar("SELECT version FROM clients WHERE id = $1 FOR UPDATE")
                .bind(order.client_id.0)
                .fetch_optional(&mut *tx)
                .await
                .map_err(store_error)?;
        if version != Some(client_version) {
            return Err(StoreError::Stale);
        }
        let role: Option<String> =
            sqlx::query_scalar("SELECT role FROM staff_accounts WHERE id = $1 FOR SHARE")
                .bind(assignee.id.0)
                .fetch_optional(&mut *tx)
                .await
                .map_err(store_error)?;
        if role.as_deref() != Some(assignee.role.as_str()) {
            return Err(StoreError::Stale);
        }
        // Foreign keys protect the final client/device pair; uniqueness is checked by UPDATE.
        save_order(&mut tx, actor, order, changes).await?;
        tx.commit().await.map_err(store_error)
    }

    async fn find_order(&self, id: OrderId) -> Result<Option<RepairOrder>, StoreError> {
        sqlx::query_as::<_, OrderRow>(concat!(
            "SELECT ",
            columns!(),
            " FROM repair_orders WHERE id = $1"
        ))
        .bind(id.0)
        .fetch_optional(self.pool())
        .await
        .and_then(|row| row.map(RepairOrder::try_from).transpose())
        .map_err(store_error)
    }

    async fn list_orders(&self, filter: &OrderFilter) -> Result<Vec<RepairOrder>, StoreError> {
        sqlx::query_as::<_, OrderRow>(concat!(
            "SELECT ",
            columns!(),
            " FROM repair_orders
             WHERE ($1::text IS NULL OR status = $1)
               AND ($2::bigint IS NULL OR client_id = $2)
               AND ($3::text IS NULL OR ticket_number = $3)
             ORDER BY id"
        ))
        .bind(filter.status.map(OrderStatus::as_str))
        .bind(filter.client_id.map(|id| id.0))
        .bind(filter.ticket_number.as_deref())
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?
        .into_iter()
        .map(|row| RepairOrder::try_from(row).map_err(store_error))
        .collect()
    }
}

/// The common versioned order write, including its audit, inside the caller's transaction.
async fn save_order(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: StaffId,
    order: &RepairOrder,
    changes: &[FieldChange],
) -> Result<(), StoreError> {
    let result = sqlx::query("UPDATE repair_orders SET status = $3, diagnostic_findings = $4, estimate_parts_cents = $5, estimate_labor_cents = $6, client_decision = $7, client_decided_at = $8, used_parts_cents = $9, completed_labor_cents = $10, repair_checked = $11, payment_cents = $12, paid_at = $13, picked_up_at = $14, labor_discount_cents = $15, settlement_cents = $16, cancelled_from = $17, ticket_number = $18, client_id = $19, device_id = $20, assigned_technician_id = $21, version = version + 1 WHERE id = $1 AND version = $2")
        .bind(order.id.0).bind(order.version).bind(order.status.as_str())
        .bind(order.diagnostic_findings.as_ref().map(DiagnosticFindings::as_str))
        .bind(order.estimate.map(|e| e.parts.get())).bind(order.estimate.map(|e| e.labor.get()))
        .bind(order.client_decision.map(|d| d.decision.as_str())).bind(order.client_decision.map(|d| d.decided_at))
        .bind(order.used_parts.get()).bind(order.completed_labor.get()).bind(order.repair_checked)
        .bind(order.payment.map(|p| p.amount.get())).bind(order.payment.map(|p| p.paid_at))
        .bind(order.picked_up_at).bind(order.final_settlement.map(|s| s.labor_discount.get()))
        .bind(order.final_settlement.map(|s| s.total.get())).bind(order.cancelled_from.map(OrderStatus::as_str))
        .bind(order.ticket_number.as_str()).bind(order.client_id.0).bind(order.device_id.0).bind(order.assigned_technician_id.0)
        .execute(&mut **tx).await.map_err(store_error)?;
    if result.rows_affected() == 0 {
        return Err(StoreError::Stale);
    }
    insert_audit(tx, actor, RecordType::RepairOrder, order.id.0, changes)
        .await
        .map_err(store_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixops_domain::client::LoyaltyTier;
    use sqlx::PgPool;
    use std::time::{Duration, Instant};

    async fn correction_fixture(pool: &PgPool) -> (PgStore, RepairOrder, StaffAccount) {
        use fixops_app::staff::StaffStore;
        let id: i64 = sqlx::query_scalar("INSERT INTO staff_accounts (username, password_hash, role) VALUES ('tech', 'unused', 'Technician') RETURNING id")
            .fetch_one(pool).await.unwrap();
        let client: i64 = sqlx::query_scalar(
            "INSERT INTO clients (name, phone) VALUES ('Bob', '123') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        let device: i64 = sqlx::query_scalar("INSERT INTO devices (client_id, device_type, serial_number, reported_symptoms) VALUES ($1, 'Laptop', 'SN1', 'No power') RETURNING id")
            .bind(client).fetch_one(pool).await.unwrap();
        let store = PgStore::new(pool.clone());
        let intake = NewRepairOrder {
            ticket_number: TicketNumber::parse("T1").unwrap(),
            client_id: ClientId(client),
            device_id: DeviceId(device),
            assigned_technician_id: StaffId(id),
        };
        let order = store
            .create_order(StaffId(id), &intake, &intake.changes())
            .await
            .unwrap();
        let staff = store.find_staff(StaffId(id)).await.unwrap().unwrap();
        (store, order, staff)
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn correction_rejects_stale_links_versions_and_rolls_back_failed_audit(pool: PgPool) {
        let (store, current, staff) = correction_fixture(&pool).await;
        let mut candidate = current.clone();
        candidate.ticket_number = TicketNumber::parse("CORRECTED").unwrap();
        let (candidate, changes) = current.correct(candidate, LoyaltyTier::Standard).unwrap();
        // Audit FK failure occurs after UPDATE and must roll back both the data and version.
        assert!(matches!(
            store
                .correct_order(StaffId(i64::MAX), &candidate, 0, &staff, &changes)
                .await,
            Err(StoreError::MissingReference { .. })
        ));
        assert_eq!(
            store.find_order(current.id).await.unwrap().unwrap(),
            current
        );
        let mut invalid_link = candidate.clone();
        invalid_link.device_id = DeviceId(i64::MAX);
        assert!(matches!(
            store
                .correct_order(staff.id, &invalid_link, 0, &staff, &changes)
                .await,
            Err(StoreError::MissingReference { .. })
        ));
        assert_eq!(
            store.find_order(current.id).await.unwrap().unwrap(),
            current
        );
        store
            .correct_order(staff.id, &candidate, 0, &staff, &changes)
            .await
            .unwrap();
        let saved = store.find_order(current.id).await.unwrap().unwrap();
        assert_eq!(saved.version, current.version + 1);
        assert!(matches!(
            store
                .correct_order(staff.id, &candidate, 0, &staff, &changes)
                .await,
            Err(StoreError::Stale)
        ));
        assert_eq!(store.find_order(current.id).await.unwrap().unwrap(), saved);
        let rows: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM audit_log WHERE record_type = 'RepairOrder' AND record_id = $1",
        )
        .bind(current.id.0)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(rows, 6); // Five intake fields and exactly one successful correction.
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn correction_waits_for_concurrent_assignee_role_change(pool: PgPool) {
        let (store, current, staff) = correction_fixture(&pool).await;
        let mut candidate = current.clone();
        candidate.ticket_number = TicketNumber::parse("CORRECTED").unwrap();
        let (candidate, changes) = current.correct(candidate, LoyaltyTier::Standard).unwrap();
        let mut tx = pool.begin().await.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE staff_accounts SET role = 'Administrator' WHERE id = $1")
            .bind(staff.id.0)
            .execute(&mut *tx)
            .await
            .unwrap();
        let writer = store.clone();
        let task = tokio::spawn(async move {
            writer
                .correct_order(staff.id, &candidate, 0, &staff, &changes)
                .await
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid)))")
                .bind(pid).fetch_one(&pool).await.unwrap();
            if waiting {
                break;
            }
            assert!(
                !task.is_finished(),
                "correction passed an uncommitted role change"
            );
            assert!(
                Instant::now() < deadline,
                "correction did not wait for the staff lock"
            );
            tokio::task::yield_now().await;
        }
        tx.commit().await.unwrap();
        assert!(matches!(task.await.unwrap(), Err(StoreError::Stale)));
        assert_eq!(
            store.find_order(current.id).await.unwrap().unwrap(),
            current
        );
        let rows: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM audit_log WHERE record_type = 'RepairOrder' AND record_id = $1",
        )
        .bind(current.id.0)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(rows, 5);
    }

    /// A tier UPDATE that already holds the client lock must finish before settlement
    /// validates its snapshot. Merely reading the version without a lock fails this test.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn settlement_waits_for_concurrent_tier_update(pool: PgPool) {
        let actor: i64 = sqlx::query_scalar("INSERT INTO staff_accounts (username, password_hash, role) VALUES ('tech', 'unused', 'Technician') RETURNING id")
            .fetch_one(&pool).await.unwrap();
        let client: i64 = sqlx::query_scalar(
            "INSERT INTO clients (name, phone) VALUES ('Bob', '123') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let device: i64 = sqlx::query_scalar("INSERT INTO devices (client_id, device_type, serial_number, reported_symptoms) VALUES ($1, 'Laptop', 'SN1', 'No power') RETURNING id")
            .bind(client).fetch_one(&pool).await.unwrap();
        let store = PgStore::new(pool.clone());
        let order = store
            .create_order(
                StaffId(actor),
                &NewRepairOrder {
                    ticket_number: TicketNumber::parse("T1").unwrap(),
                    client_id: ClientId(client),
                    device_id: DeviceId(device),
                    assigned_technician_id: StaffId(actor),
                },
                &[FieldChange::created("status", OrderStatus::Received)],
            )
            .await
            .unwrap();
        let (cancelled, changes) = order.cancel(LoyaltyTier::Standard).unwrap();
        let mut tx = pool.begin().await.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE clients SET loyalty_tier = 'Gold', version = version + 1 WHERE id = $1",
        )
        .bind(client)
        .execute(&mut *tx)
        .await
        .unwrap();
        let writer = store.clone();
        let task = tokio::spawn(async move {
            writer
                .settle_order(StaffId(actor), &cancelled, 0, &changes)
                .await
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid)))")
                .bind(pid).fetch_one(&pool).await.unwrap();
            if waiting {
                break;
            }
            assert!(
                !task.is_finished(),
                "settlement passed an uncommitted tier change"
            );
            assert!(
                Instant::now() < deadline,
                "settlement did not wait on the client lock"
            );
            tokio::task::yield_now().await;
        }
        tx.commit().await.unwrap();
        let result = task.await.unwrap();
        assert!(matches!(result, Err(StoreError::Stale)), "{result:?}");
        assert_eq!(store.find_order(order.id).await.unwrap().unwrap(), order);
        let rows: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM audit_log WHERE record_type = 'RepairOrder' AND record_id = $1",
        )
        .bind(order.id.0)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(rows, 1);
    }
}
