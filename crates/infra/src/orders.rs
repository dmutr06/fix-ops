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
         payment_cents, paid_at, picked_up_at, labor_discount_cents, settlement_cents, version"
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
