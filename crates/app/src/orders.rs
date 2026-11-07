//! Repair-order intake and reading (REQ-F-005).

use fixops_domain::audit::FieldChange;
use fixops_domain::client::LoyaltyTier;
use fixops_domain::id::{ClientId, DeviceId, OrderId, StaffId};
use fixops_domain::order::{
    ClientDecision, DiagnosticFindings, Estimate, GuardViolation, NewRepairOrder, OrderStatus,
    RepairOrder, RepairWork, TicketNumber,
};
use fixops_domain::pricing::{Settlement, current_settlement};
use fixops_domain::staff::{Action, Actor};

use crate::clients::ClientStore;
use crate::devices::DeviceStore;
use crate::staff::StaffStore;
use crate::{App, AppError, AppResult, StoreError};

/// What reception enters to open an order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderIntake {
    pub ticket_number: TicketNumber,
    pub client_id: ClientId,
    pub device_id: DeviceId,
    pub assigned_technician_id: StaffId,
}

/// An order together with its derived settlement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderView {
    pub order: RepairOrder,
    pub settlement: Settlement,
}

impl From<RepairOrder> for OrderView {
    fn from(order: RepairOrder) -> Self {
        // Received and Diagnosing orders owe nothing while open, whatever the tier.
        let settlement = current_settlement(&order, LoyaltyTier::Standard);
        Self { order, settlement }
    }
}

/// Which orders to list. Unset fields match everything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrderFilter {
    pub status: Option<OrderStatus>,
    pub client_id: Option<ClientId>,
    /// Exact ticket number.
    pub ticket_number: Option<String>,
}

/// Port: storage of repair orders.
pub trait OrderStore: Send + Sync {
    /// Inserts an order in `Received` with its audit `changes` in one transaction. A taken
    /// ticket number fails with [`StoreError::Duplicate`].
    fn create_order(
        &self,
        actor: StaffId,
        order: &NewRepairOrder,
        changes: &[FieldChange],
    ) -> impl Future<Output = Result<RepairOrder, StoreError>> + Send;

    fn update_order(
        &self,
        actor: StaffId,
        order: &RepairOrder,
        changes: &[FieldChange],
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn find_order(
        &self,
        id: OrderId,
    ) -> impl Future<Output = Result<Option<RepairOrder>, StoreError>> + Send;

    /// Matching orders, ordered by id.
    fn list_orders(
        &self,
        filter: &OrderFilter,
    ) -> impl Future<Output = Result<Vec<RepairOrder>, StoreError>> + Send;
}

impl<S: OrderStore + ClientStore + DeviceStore + StaffStore> App<S> {
    /// Opens an order in `Received`. Unknown links are `404`; a device of another client, an
    /// assignee who is not a Technician, or a taken ticket number is `409`.
    pub async fn create_order(&self, actor: Actor, intake: OrderIntake) -> AppResult<OrderView> {
        crate::authorize(actor, Action::CreateRepairOrder)?;
        let client = self.store.find_client(intake.client_id).await?;
        let client = client.ok_or_else(|| missing_link("client_id"))?;
        let device = self.store.find_device(intake.device_id).await?;
        let device = device.ok_or_else(|| missing_link("device_id"))?;
        let assignee = self.store.find_staff(intake.assigned_technician_id).await?;
        let assignee = assignee.ok_or_else(|| missing_link("assigned_technician_id"))?;

        let order = NewRepairOrder::intake(intake.ticket_number, &client, &device, &assignee)
            .map_err(|err| AppError::conflict(err.to_string()).with("field", err.field()))?;
        let created = self
            .store
            .create_order(actor.id, &order, &order.changes())
            .await
            .map_err(|err| match err {
                StoreError::Duplicate { .. } => AppError::conflict("ticket_number already exists")
                    .with("ticket_number", &order.ticket_number),
                other => other.into(),
            })?;
        Ok(created.into())
    }

    pub async fn record_diagnosis(
        &self,
        actor: Actor,
        id: OrderId,
        findings: Option<DiagnosticFindings>,
    ) -> AppResult<OrderView> {
        crate::authorize(actor, Action::RecordDiagnosis)?;
        let current = self
            .store
            .find_order(id)
            .await?
            .ok_or_else(AppError::not_found)?;
        let (order, changes) = current.record_diagnosis(findings).map_err(guard)?;
        self.save_order(actor, &order, &changes).await?;
        Ok(order.into())
    }

    pub async fn record_estimate(
        &self,
        actor: Actor,
        id: OrderId,
        estimate: Estimate,
    ) -> AppResult<OrderView> {
        crate::authorize(actor, Action::RecordEstimate)?;
        let current = self
            .store
            .find_order(id)
            .await?
            .ok_or_else(AppError::not_found)?;
        let (order, changes) = current.record_estimate(estimate).map_err(guard)?;
        self.save_order(actor, &order, &changes).await?;
        Ok(order.into())
    }

    pub async fn record_decision(
        &self,
        actor: Actor,
        id: OrderId,
        decision: ClientDecision,
    ) -> AppResult<OrderView> {
        crate::authorize(actor, Action::RecordClientDecision)?;
        let current = self
            .store
            .find_order(id)
            .await?
            .ok_or_else(AppError::not_found)?;
        let (order, changes) = current.record_decision(decision).map_err(guard)?;
        self.save_order(actor, &order, &changes).await?;
        Ok(order.into())
    }

    pub async fn update_repair_work(
        &self,
        actor: Actor,
        id: OrderId,
        work: RepairWork,
    ) -> AppResult<OrderView> {
        crate::authorize(actor, Action::UpdateRepairWork)?;
        let current = self
            .store
            .find_order(id)
            .await?
            .ok_or_else(AppError::not_found)?;
        let (order, changes) = current.record_repair_work(work).map_err(|e| {
            let mut error = guard(e).with("current_status", current.status);
            if let Some(value) = e.guard() {
                error = error.with("guard", value);
            }
            if let Some(value) = e.requested_status() {
                error = error.with("requested_status", value);
            }
            error
        })?;
        let view = self.view_order(order).await?;
        self.save_order(actor, &view.order, &changes).await?;
        Ok(view)
    }

    async fn view_order(&self, order: RepairOrder) -> AppResult<OrderView> {
        let client = self
            .store
            .find_client(order.client_id)
            .await?
            .ok_or_else(AppError::not_found)?;
        let settlement = current_settlement(&order, client.loyalty_tier);
        Ok(OrderView { order, settlement })
    }

    async fn save_order(
        &self,
        actor: Actor,
        order: &RepairOrder,
        changes: &[FieldChange],
    ) -> AppResult<()> {
        if !changes.is_empty() {
            self.store.update_order(actor.id, order, changes).await?;
        }
        Ok(())
    }

    pub async fn get_order(&self, actor: Actor, id: OrderId) -> AppResult<OrderView> {
        crate::authorize(actor, Action::GetRepairOrder)?;
        let order = self.store.find_order(id).await?;
        self.view_order(order.ok_or_else(AppError::not_found)?)
            .await
    }

    pub async fn list_orders(
        &self,
        actor: Actor,
        filter: OrderFilter,
    ) -> AppResult<Vec<OrderView>> {
        crate::authorize(actor, Action::ListRepairOrders)?;
        let orders = self.store.list_orders(&filter).await?;
        let mut views = Vec::with_capacity(orders.len());
        for order in orders {
            views.push(self.view_order(order).await?);
        }
        Ok(views)
    }
}

fn missing_link(field: &'static str) -> AppError {
    AppError::not_found().with("field", field)
}

fn guard(error: GuardViolation) -> AppError {
    AppError::guard(error.to_string())
}
