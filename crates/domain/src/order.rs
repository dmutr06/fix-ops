//! Repair orders: the aggregate, its status and intake (REQ-F-005, SRS §3.1).

use std::fmt;

use time::{OffsetDateTime, UtcOffset};

use crate::audit::FieldChange;
use crate::client::{Client, LoyaltyTier};
use crate::device::Device;
use crate::error::{InvalidField, required_text};
use crate::id::{ClientId, DeviceId, OrderId, StaffId};
use crate::money::Cents;
use crate::pricing::{Settlement, current_settlement};
use crate::staff::{Role, StaffAccount};

/// Lifecycle state from SRS §3.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderStatus {
    Received,
    Diagnosing,
    InProgress,
    Ready,
    Completed,
    Cancelled,
}

impl OrderStatus {
    pub const ALL: [Self; 6] = [
        Self::Received,
        Self::Diagnosing,
        Self::InProgress,
        Self::Ready,
        Self::Completed,
        Self::Cancelled,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Received => "Received",
            Self::Diagnosing => "Diagnosing",
            Self::InProgress => "InProgress",
            Self::Ready => "Ready",
            Self::Completed => "Completed",
            Self::Cancelled => "Cancelled",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == raw)
            .ok_or(InvalidField::new(
                "status",
                "status must be Received, Diagnosing, InProgress, Ready, Completed or Cancelled",
            ))
    }
}

impl OrderStatus {
    /// Whether SRS §3.1 lists `self → next`. This table is the only place transitions are
    /// allowed; every other pair is rejected, whoever asks (ADD-001).
    pub const fn can_move_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Received, Self::Diagnosing | Self::Cancelled)
                | (Self::Diagnosing, Self::InProgress | Self::Cancelled)
                | (Self::InProgress, Self::Ready | Self::Cancelled)
                | (Self::Ready, Self::Completed)
        )
    }

    /// Succeeds when `self → next` is a listed transition.
    pub fn check_move(self, next: Self) -> Result<(), GuardViolation> {
        if self.can_move_to(next) {
            Ok(())
        } else {
            Err(GuardViolation::Transition {
                from: self,
                to: next,
            })
        }
    }
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A ticket number: trimmed and non-empty. Uniqueness is checked by the store.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TicketNumber(String);

impl TicketNumber {
    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        required_text(
            "ticket_number",
            "ticket_number is required and cannot be empty",
            raw,
        )
        .map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TicketNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The client's answer to an estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Decision {
    Approved,
    Declined,
}

impl Decision {
    pub const ALL: [Self; 2] = [Self::Approved, Self::Declined];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "Approved",
            Self::Declined => "Declined",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        Self::ALL
            .into_iter()
            .find(|decision| decision.as_str() == raw)
            .ok_or(InvalidField::new(
                "decision",
                "decision must be Approved or Declined",
            ))
    }
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The Technician's saved diagnosis result: trimmed and non-empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticFindings(String);

impl DiagnosticFindings {
    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        required_text(
            "diagnostic_findings",
            "diagnostic_findings cannot be empty",
            raw,
        )
        .map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiagnosticFindings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Expected parts and labor charges. Not an incurred cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Estimate {
    pub parts: Cents,
    pub labor: Cents,
}

/// A recorded client decision and its time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientDecision {
    pub decision: Decision,
    pub decided_at: OffsetDateTime,
}

/// A recorded payment and its time. Payment is recorded, not processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Payment {
    pub amount: Cents,
    pub paid_at: OffsetDateTime,
}

/// A repair order with every field the contract exposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairOrder {
    pub id: OrderId,
    pub ticket_number: TicketNumber,
    pub client_id: ClientId,
    pub device_id: DeviceId,
    pub assigned_technician_id: StaffId,
    pub status: OrderStatus,
    pub diagnostic_findings: Option<DiagnosticFindings>,
    pub estimate: Option<Estimate>,
    pub client_decision: Option<ClientDecision>,
    pub used_parts: Cents,
    pub completed_labor: Cents,
    pub repair_checked: bool,
    pub payment: Option<Payment>,
    pub picked_up_at: Option<OffsetDateTime>,
    /// The settlement fixed when the order reached `Completed` or `Cancelled`; `None` while open.
    pub final_settlement: Option<Settlement>,
    /// Source state retained for pricing future corrections of a cancelled order.
    pub cancelled_from: Option<OrderStatus>,
    /// Optimistic concurrency token, increased by every saved change.
    pub version: i64,
}

/// The state `PUT /repair` asks for: start or continue the repair, or finish it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RepairStage {
    InProgress,
    Ready,
}

impl RepairStage {
    pub const ALL: [Self; 2] = [Self::InProgress, Self::Ready];

    pub const fn status(self) -> OrderStatus {
        match self {
            Self::InProgress => OrderStatus::InProgress,
            Self::Ready => OrderStatus::Ready,
        }
    }

    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        Self::ALL
            .into_iter()
            .find(|stage| stage.status().as_str() == raw)
            .ok_or(InvalidField::new(
                "status",
                "status must be InProgress or Ready",
            ))
    }
}

/// A repair work update (REQ-F-009–011). `None` keeps a recorded value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairWork {
    pub stage: RepairStage,
    pub used_parts: Option<Cents>,
    pub completed_labor: Option<Cents>,
    pub repair_checked: Option<bool>,
}

/// A lifecycle rule blocked a change (SRS §3.1). The order is left as it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GuardViolation {
    /// SRS §3.1 does not list this transition.
    #[error("Cannot move an order from {from} to {to}")]
    Transition { from: OrderStatus, to: OrderStatus },
    /// The action is only available while the order is in `expected`.
    #[error("Only a {expected} order accepts this action; the order is {status}")]
    Status {
        status: OrderStatus,
        expected: OrderStatus,
    },
    #[error("Diagnostic findings must be saved before preparing an estimate")]
    FindingsMissing,
    #[error("An estimate must be prepared before a client decision is recorded")]
    EstimateMissing,
    #[error("A client decision is already recorded for this estimate")]
    DecisionRecorded,
    #[error("Cannot enter InProgress without recorded client approval on estimate")]
    ApprovalMissing,
    #[error("Cannot enter Ready without a recorded repair check")]
    RepairCheckMissing,
    #[error("Incurred charges exceed the supported amount")]
    ChargesOverflow,
    #[error("Decision time cannot be represented as RFC 3339")]
    InvalidDecisionTime,
    #[error("Payment or pickup time cannot be represented as RFC 3339")]
    InvalidSettlementTime,
    #[error("Cannot complete order without a recorded full payment")]
    PaymentMissing,
    #[error(
        "Recorded payment ({paid} cents) does not equal full settlement amount ({total} cents)"
    )]
    PaymentMismatch { paid: Cents, total: Cents },
    #[error("Cannot complete order without a recorded device pickup")]
    PickupMissing,
}

impl GuardViolation {
    /// The recorded value the rule requires, as the contract names it in `details.guard`.
    pub const fn guard(self) -> Option<&'static str> {
        match self {
            Self::ApprovalMissing => Some("client_decision.Approved"),
            Self::RepairCheckMissing => Some("repair_checked"),
            Self::PaymentMissing => Some("payment"),
            Self::PickupMissing => Some("pickup"),
            _ => None,
        }
    }

    /// The state a rejected transition asked for.
    pub const fn requested_status(self) -> Option<OrderStatus> {
        match self {
            Self::Transition { to, .. } => Some(to),
            _ => None,
        }
    }
}

/// The result of a successful rule: the new order and one change per changed field.
pub type Changed = (RepairOrder, Vec<FieldChange>);

impl RepairOrder {
    /// Records or replaces the payment on a Ready order (REQ-F-012).
    pub fn record_payment(&self, payment: Payment) -> Result<Changed, GuardViolation> {
        self.require_ready()?;
        let (paid_at, timestamp) = settlement_time(payment.paid_at)?;
        let payment = Payment { paid_at, ..payment };
        let previous_time = self
            .payment
            .map(|p| settlement_time(p.paid_at).map(|(_, s)| s))
            .transpose()?;
        let mut changes = Vec::new();
        changes.extend(FieldChange::diff_optional(
            "payment.amount_cents",
            self.payment.as_ref().map(|p| &p.amount),
            Some(&payment.amount),
        ));
        changes.extend(FieldChange::diff_optional(
            "payment.paid_at",
            previous_time.as_ref(),
            Some(&timestamp),
        ));
        let mut order = self.clone();
        order.payment = Some(payment);
        Ok((order, changes))
    }

    /// Records pickup independently of payment on a Ready order (REQ-F-013).
    pub fn record_pickup(&self, picked_up_at: OffsetDateTime) -> Result<Changed, GuardViolation> {
        self.require_ready()?;
        let (picked_up_at, timestamp) = settlement_time(picked_up_at)?;
        let previous = self
            .picked_up_at
            .map(|t| settlement_time(t).map(|(_, s)| s))
            .transpose()?;
        let changes =
            FieldChange::diff_optional("pickup.picked_up_at", previous.as_ref(), Some(&timestamp))
                .into_iter()
                .collect();
        let mut order = self.clone();
        order.picked_up_at = Some(picked_up_at);
        Ok((order, changes))
    }

    /// Freezes the current settlement after exact payment and pickup (REQ-F-013, 016).
    pub fn complete(&self, tier: LoyaltyTier) -> Result<Changed, GuardViolation> {
        self.status.check_move(OrderStatus::Completed)?;
        let settlement = current_settlement(self, tier);
        let payment = self.payment.ok_or(GuardViolation::PaymentMissing)?;
        if payment.amount != settlement.total {
            return Err(GuardViolation::PaymentMismatch {
                paid: payment.amount,
                total: settlement.total,
            });
        }
        if self.picked_up_at.is_none() {
            return Err(GuardViolation::PickupMissing);
        }
        Ok(self.finalize(OrderStatus::Completed, settlement))
    }

    /// Cancels from the three eligible source states, preserving the pricing stage.
    pub fn cancel(&self, tier: LoyaltyTier) -> Result<Changed, GuardViolation> {
        self.status.check_move(OrderStatus::Cancelled)?;
        let settlement = match self.status {
            OrderStatus::Diagnosing if self.diagnostic_findings.is_some() => Settlement {
                labor_discount: Cents::ZERO,
                total: Cents::new("settlement_cents", 3000)
                    .map_err(|_| GuardViolation::ChargesOverflow)?,
            },
            _ => current_settlement(self, tier),
        };
        Ok(self.finalize(OrderStatus::Cancelled, settlement))
    }

    fn require_ready(&self) -> Result<(), GuardViolation> {
        if self.status != OrderStatus::Ready {
            return Err(GuardViolation::Status {
                status: self.status,
                expected: OrderStatus::Ready,
            });
        }
        Ok(())
    }

    fn finalize(&self, status: OrderStatus, settlement: Settlement) -> Changed {
        let mut order = self.clone();
        order.status = status;
        order.final_settlement = Some(settlement);
        if status == OrderStatus::Cancelled {
            order.cancelled_from = Some(self.status);
        }
        // The final amounts are newly stored snapshots; open totals were derived only.
        let changes = vec![
            FieldChange::new(
                "status",
                Some(self.status.to_string()),
                Some(status.to_string()),
            ),
            FieldChange::created("labor_discount_cents", settlement.labor_discount),
            FieldChange::created("settlement_cents", settlement.total),
        ];
        (order, changes)
    }

    /// Starts diagnosis on a `Received` order and saves `findings` when given (REQ-F-006).
    /// While `Diagnosing` and before a client decision, findings can be saved or revised.
    /// `None` keeps what is saved.
    pub fn record_diagnosis(
        &self,
        findings: Option<DiagnosticFindings>,
    ) -> Result<Changed, GuardViolation> {
        if self.status != OrderStatus::Received && self.status != OrderStatus::Diagnosing {
            return Err(GuardViolation::Transition {
                from: self.status,
                to: OrderStatus::Diagnosing,
            });
        }
        if self.client_decision.is_some() {
            return Err(GuardViolation::DecisionRecorded);
        }
        let mut order = self.clone();
        let mut changes = Vec::new();
        if order.status == OrderStatus::Received {
            changes.push(FieldChange::new(
                "status",
                Some("Received".into()),
                Some("Diagnosing".into()),
            ));
            order.status = OrderStatus::Diagnosing;
        }
        if let Some(findings) = findings {
            changes.extend(FieldChange::diff_optional(
                "diagnostic_findings",
                order.diagnostic_findings.as_ref(),
                Some(&findings),
            ));
            order.diagnostic_findings = Some(findings);
        }
        Ok((order, changes))
    }

    /// Saves or revises the itemized estimate: only after findings are saved and before the
    /// client decides (REQ-F-007).
    pub fn record_estimate(&self, estimate: Estimate) -> Result<Changed, GuardViolation> {
        if self.client_decision.is_some() {
            return Err(GuardViolation::DecisionRecorded);
        }
        if self.diagnostic_findings.is_none() {
            return Err(GuardViolation::FindingsMissing);
        }
        if self.status != OrderStatus::Diagnosing {
            return Err(GuardViolation::Status {
                status: self.status,
                expected: OrderStatus::Diagnosing,
            });
        }
        let mut order = self.clone();
        let mut changes = Vec::new();
        changes.extend(FieldChange::diff_optional(
            "estimate.parts_cents",
            self.estimate.as_ref().map(|e| &e.parts),
            Some(&estimate.parts),
        ));
        changes.extend(FieldChange::diff_optional(
            "estimate.labor_cents",
            self.estimate.as_ref().map(|e| &e.labor),
            Some(&estimate.labor),
        ));
        order.estimate = Some(estimate);
        Ok((order, changes))
    }

    /// Records the client's decision on the estimate and its time, once (REQ-F-008).
    pub fn record_decision(&self, decision: ClientDecision) -> Result<Changed, GuardViolation> {
        if self.client_decision.is_some() {
            return Err(GuardViolation::DecisionRecorded);
        }
        if self.estimate.is_none() {
            return Err(GuardViolation::EstimateMissing);
        }
        if self.status != OrderStatus::Diagnosing {
            return Err(GuardViolation::Status {
                status: self.status,
                expected: OrderStatus::Diagnosing,
            });
        }
        let mut order = self.clone();
        let decision = ClientDecision {
            decided_at: decision
                .decided_at
                .to_offset(UtcOffset::UTC)
                .truncate_to_microsecond(),
            ..decision
        };
        let timestamp = decision
            .decided_at
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|_| GuardViolation::InvalidDecisionTime)?;
        let changes = vec![
            FieldChange::created("client_decision.decision", decision.decision),
            FieldChange::created("client_decision.decided_at", timestamp),
        ];
        order.client_decision = Some(decision);
        Ok((order, changes))
    }

    /// Starts the repair, records incurred charges and the repair check, or finishes the
    /// repair (REQ-F-009–011, SRS §3.1).
    pub fn record_repair_work(&self, work: RepairWork) -> Result<Changed, GuardViolation> {
        let target = work.stage.status();
        if self.status != OrderStatus::InProgress || target != OrderStatus::InProgress {
            self.status.check_move(target)?;
        }
        if self.status == OrderStatus::Diagnosing
            && (self.estimate.is_none()
                || !matches!(
                    self.client_decision,
                    Some(ClientDecision {
                        decision: Decision::Approved,
                        ..
                    })
                ))
        {
            return Err(GuardViolation::ApprovalMissing);
        }
        let mut order = self.clone();
        order.status = target;
        order.used_parts = work.used_parts.unwrap_or(self.used_parts);
        order.completed_labor = work.completed_labor.unwrap_or(self.completed_labor);
        order.repair_checked = work.repair_checked.unwrap_or(self.repair_checked);
        if target == OrderStatus::Ready && !order.repair_checked {
            return Err(GuardViolation::RepairCheckMissing);
        }
        if order
            .used_parts
            .get()
            .checked_add(order.completed_labor.get())
            .is_none()
        {
            return Err(GuardViolation::ChargesOverflow);
        }
        let mut changes = Vec::new();
        changes.extend(FieldChange::diff("status", &self.status, &order.status));
        changes.extend(FieldChange::diff(
            "used_parts_cents",
            &self.used_parts,
            &order.used_parts,
        ));
        changes.extend(FieldChange::diff(
            "completed_labor_cents",
            &self.completed_labor,
            &order.completed_labor,
        ));
        changes.extend(FieldChange::diff(
            "repair_checked",
            &self.repair_checked,
            &order.repair_checked,
        ));
        Ok((order, changes))
    }
}

/// Match PostgreSQL precision before returning or auditing a recorded timestamp.
fn settlement_time(value: OffsetDateTime) -> Result<(OffsetDateTime, String), GuardViolation> {
    let value = value.to_offset(UtcOffset::UTC).truncate_to_microsecond();
    let text = value
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| GuardViolation::InvalidSettlementTime)?;
    Ok((value, text))
}

/// Why an intake was refused although every linked record exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IntakeError {
    #[error("device does not belong to the client")]
    DeviceOfAnotherClient,
    #[error("assigned staff account is not a Technician")]
    AssigneeNotTechnician,
}

impl IntakeError {
    /// The request field that holds the rejected link.
    pub const fn field(self) -> &'static str {
        match self {
            Self::DeviceOfAnotherClient => "device_id",
            Self::AssigneeNotTechnician => "assigned_technician_id",
        }
    }
}

/// A repair order about to be created in `Received` (REQ-F-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRepairOrder {
    pub ticket_number: TicketNumber,
    pub client_id: ClientId,
    pub device_id: DeviceId,
    pub assigned_technician_id: StaffId,
}

impl NewRepairOrder {
    /// Every new order starts here.
    pub const STATUS: OrderStatus = OrderStatus::Received;

    /// Links an order to existing records: the device must belong to the client, and the
    /// assignee must hold the Technician role.
    pub fn intake(
        ticket_number: TicketNumber,
        client: &Client,
        device: &Device,
        assignee: &StaffAccount,
    ) -> Result<Self, IntakeError> {
        if device.client_id != client.id {
            return Err(IntakeError::DeviceOfAnotherClient);
        }
        if assignee.role != Role::Technician {
            return Err(IntakeError::AssigneeNotTechnician);
        }
        Ok(Self {
            ticket_number,
            client_id: client.id,
            device_id: device.id,
            assigned_technician_id: assignee.id,
        })
    }

    /// Audit changes for the creation: the ticket, the three links and the starting status.
    pub fn changes(&self) -> Vec<FieldChange> {
        vec![
            FieldChange::created("ticket_number", &self.ticket_number),
            FieldChange::created("client_id", self.client_id),
            FieldChange::created("device_id", self.device_id),
            FieldChange::created("assigned_technician_id", self.assigned_technician_id),
            FieldChange::created("status", Self::STATUS),
        ]
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::client::{ClientName, LoyaltyTier, Phone};
    use crate::device::{DeviceType, SerialNumber, Symptoms};
    use crate::staff::Username;
    use time::{Date, Month, PrimitiveDateTime, Time};

    /// 2026-10-21 at `hour:minute` UTC.
    pub(crate) fn october_21(hour: u8, minute: u8) -> OffsetDateTime {
        PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::October, 21).unwrap(),
            Time::from_hms(hour, minute, 0).unwrap(),
        )
        .assume_utc()
    }

    /// A freshly received order with nothing recorded yet.
    pub(crate) fn received() -> RepairOrder {
        RepairOrder {
            id: OrderId(100),
            ticket_number: TicketNumber::parse("FIX-2026-0001").unwrap(),
            client_id: ClientId(10),
            device_id: DeviceId(20),
            assigned_technician_id: StaffId(2),
            status: OrderStatus::Received,
            diagnostic_findings: None,
            estimate: None,
            client_decision: None,
            used_parts: Cents::ZERO,
            completed_labor: Cents::ZERO,
            repair_checked: false,
            payment: None,
            picked_up_at: None,
            final_settlement: None,
            cancelled_from: None,
            version: 1,
        }
    }

    #[test]
    fn corrections_validate_the_combined_candidate_without_mutating_current() {
        let current = received();
        let mut candidate = current.clone();
        candidate.status = OrderStatus::Diagnosing;
        candidate.diagnostic_findings = Some(DiagnosticFindings::parse("Power failure").unwrap());
        candidate.estimate = Some(Estimate {
            parts: Cents::ZERO,
            labor: Cents::ZERO,
        });
        let (saved, changes) = current.correct(candidate, LoyaltyTier::Standard).unwrap();
        assert_eq!(current, received());
        assert_eq!(saved.status, OrderStatus::Diagnosing);
        assert!(changes.iter().any(|c| c.field == "estimate.parts_cents"));
        let mut invalid = saved.clone();
        invalid.diagnostic_findings = None;
        assert_eq!(
            saved.correct(invalid, LoyaltyTier::Standard),
            Err(GuardViolation::FindingsMissing)
        );
    }

    #[test]
    fn legacy_cancellation_allows_metadata_but_never_guesses_pricing_source() {
        let (mut current, _) = received().cancel(LoyaltyTier::Standard).unwrap();
        current.cancelled_from = None;
        let mut metadata = current.clone();
        metadata.ticket_number = TicketNumber::parse("CORRECTED").unwrap();
        let (saved, _) = current.correct(metadata, LoyaltyTier::Gold).unwrap();
        assert_eq!(saved.final_settlement, current.final_settlement);
        assert_eq!(saved.cancelled_from, None);
        let mut financial = saved.clone();
        financial.completed_labor = Cents::new("completed_labor_cents", 1).unwrap();
        assert!(saved.correct(financial, LoyaltyTier::Gold).is_err());
        let mut findings = saved.clone();
        findings.diagnostic_findings = Some(DiagnosticFindings::parse("New evidence").unwrap());
        assert!(saved.correct(findings, LoyaltyTier::Gold).is_err());
    }

    fn client(id: i64) -> Client {
        Client {
            id: ClientId(id),
            name: ClientName::parse("Bob Vance").unwrap(),
            phone: Phone::parse("+1-555-0100").unwrap(),
            email: None,
            loyalty_tier: LoyaltyTier::Standard,
        }
    }

    fn device_of(client: i64) -> Device {
        Device {
            id: DeviceId(20),
            client_id: ClientId(client),
            device_type: DeviceType::Laptop,
            serial_number: SerialNumber::parse("SN-THINKPAD-9001").unwrap(),
            reported_symptoms: Symptoms::parse("No power").unwrap(),
        }
    }

    fn staff(role: Role) -> StaffAccount {
        StaffAccount {
            id: StaffId(2),
            username: Username::parse("max.tech").unwrap(),
            role,
            enabled: true,
        }
    }

    fn ticket() -> TicketNumber {
        TicketNumber::parse("FIX-2026-0001").unwrap()
    }

    #[test]
    fn status_parses_the_six_contract_names_only() {
        for status in OrderStatus::ALL {
            assert_eq!(OrderStatus::parse(status.as_str()), Ok(status));
        }
        for invalid in ["received", "Done", "", "In Progress"] {
            assert_eq!(OrderStatus::parse(invalid).unwrap_err().field, "status");
        }
    }

    #[test]
    fn decision_parses_the_two_contract_names_only() {
        for decision in Decision::ALL {
            assert_eq!(Decision::parse(decision.as_str()), Ok(decision));
        }
        assert_eq!(Decision::parse("approved").unwrap_err().field, "decision");
    }

    #[test]
    fn ticket_number_is_trimmed_and_required() {
        assert_eq!(
            TicketNumber::parse(" FIX-2026-0001 ").unwrap().as_str(),
            "FIX-2026-0001"
        );
        for blank in ["", "   ", "\t"] {
            assert_eq!(
                TicketNumber::parse(blank).unwrap_err().field,
                "ticket_number"
            );
        }
    }

    #[test]
    fn intake_links_the_client_its_device_and_a_technician() {
        let order = NewRepairOrder::intake(
            ticket(),
            &client(10),
            &device_of(10),
            &staff(Role::Technician),
        )
        .unwrap();
        assert_eq!(
            order,
            NewRepairOrder {
                ticket_number: ticket(),
                client_id: ClientId(10),
                device_id: DeviceId(20),
                assigned_technician_id: StaffId(2),
            }
        );
        assert_eq!(NewRepairOrder::STATUS, OrderStatus::Received);
    }

    #[test]
    fn intake_rejects_a_device_of_another_client() {
        let err = NewRepairOrder::intake(
            ticket(),
            &client(10),
            &device_of(11),
            &staff(Role::Technician),
        )
        .unwrap_err();
        assert_eq!(err, IntakeError::DeviceOfAnotherClient);
        assert_eq!(err.field(), "device_id");
    }

    /// REQ-F-005 and the glossary name an assigned *Technician*. Administrator may perform
    /// Technician actions (ADD-001 §2), but that grants permissions, not the role itself.
    #[test]
    fn intake_assigns_only_a_technician() {
        for role in [Role::Receptionist, Role::Administrator] {
            let err = NewRepairOrder::intake(ticket(), &client(10), &device_of(10), &staff(role))
                .unwrap_err();
            assert_eq!(err, IntakeError::AssigneeNotTechnician, "{role}");
            assert_eq!(err.field(), "assigned_technician_id");
        }
    }

    #[test]
    fn creation_audits_ticket_links_and_status() {
        let order = NewRepairOrder {
            ticket_number: ticket(),
            client_id: ClientId(10),
            device_id: DeviceId(20),
            assigned_technician_id: StaffId(2),
        };
        assert_eq!(
            order.changes(),
            vec![
                FieldChange::created("ticket_number", "FIX-2026-0001"),
                FieldChange::created("client_id", 10),
                FieldChange::created("device_id", 20),
                FieldChange::created("assigned_technician_id", 2),
                FieldChange::created("status", "Received"),
            ]
        );
    }

    #[test]
    fn a_received_order_has_nothing_recorded() {
        let order = received();
        assert_eq!(order.status, OrderStatus::Received);
        assert_eq!(order.used_parts, Cents::ZERO);
        assert_eq!(order.completed_labor, Cents::ZERO);
        assert_eq!(order.final_settlement, None);
    }

    use OrderStatus::{Cancelled, Completed, Diagnosing, InProgress, Ready, Received};

    #[test]
    fn lifecycle_checks_all_36_source_target_pairs() {
        let allowed = [
            (Received, Diagnosing),
            (Received, Cancelled),
            (Diagnosing, InProgress),
            (Diagnosing, Cancelled),
            (InProgress, Ready),
            (InProgress, Cancelled),
            (Ready, Completed),
        ];
        for from in OrderStatus::ALL {
            for to in OrderStatus::ALL {
                assert_eq!(
                    from.check_move(to).is_ok(),
                    allowed.contains(&(from, to)),
                    "{from} -> {to}"
                );
            }
        }
    }

    // --- Values ---

    #[test]
    fn findings_are_trimmed_and_required() {
        assert_eq!(
            DiagnosticFindings::parse("  Faulty power IC ")
                .unwrap()
                .as_str(),
            "Faulty power IC"
        );
        for blank in ["", "   ", "\n"] {
            assert_eq!(
                DiagnosticFindings::parse(blank).unwrap_err().field,
                "diagnostic_findings"
            );
        }
    }

    // --- Orders at each step ---

    fn cents(value: i64) -> Cents {
        Cents::new("x", value).unwrap()
    }

    fn findings(text: &str) -> DiagnosticFindings {
        DiagnosticFindings::parse(text).unwrap()
    }

    fn estimate(parts: i64, labor: i64) -> Estimate {
        Estimate {
            parts: cents(parts),
            labor: cents(labor),
        }
    }

    fn decision(decision: Decision) -> ClientDecision {
        ClientDecision {
            decision,
            decided_at: october_21(10, 15),
        }
    }

    fn diagnosing(saved: Option<&str>) -> RepairOrder {
        RepairOrder {
            status: Diagnosing,
            diagnostic_findings: saved.map(findings),
            ..received()
        }
    }

    fn estimated() -> RepairOrder {
        RepairOrder {
            estimate: Some(estimate(10_000, 10_000)),
            ..diagnosing(Some("Faulty power IC"))
        }
    }

    fn decided(answer: Decision) -> RepairOrder {
        RepairOrder {
            client_decision: Some(decision(answer)),
            ..estimated()
        }
    }

    fn in_progress() -> RepairOrder {
        RepairOrder {
            status: InProgress,
            ..decided(Decision::Approved)
        }
    }

    fn ready() -> RepairOrder {
        RepairOrder {
            status: Ready,
            used_parts: cents(10_000),
            completed_labor: cents(10_000),
            repair_checked: true,
            ..in_progress()
        }
    }

    fn change(field: &'static str, previous: Option<&str>, new: Option<&str>) -> FieldChange {
        FieldChange::new(field, previous.map(Into::into), new.map(Into::into))
    }

    fn work(stage: RepairStage) -> RepairWork {
        RepairWork {
            stage,
            used_parts: None,
            completed_labor: None,
            repair_checked: None,
        }
    }

    // --- Diagnosis (REQ-F-006) ---

    #[test]
    fn diagnosis_starts_on_a_received_order_with_findings() {
        let (order, changes) = received()
            .record_diagnosis(Some(findings("Faulty power IC")))
            .unwrap();
        assert_eq!(order, diagnosing(Some("Faulty power IC")));
        assert_eq!(
            changes,
            [
                change("status", Some("Received"), Some("Diagnosing")),
                change("diagnostic_findings", None, Some("Faulty power IC")),
            ]
        );
    }

    /// BDD-013: diagnosis may start before any findings are saved.
    #[test]
    fn diagnosis_starts_without_findings() {
        let (order, changes) = received().record_diagnosis(None).unwrap();
        assert_eq!(order, diagnosing(None));
        assert_eq!(
            changes,
            [change("status", Some("Received"), Some("Diagnosing"))]
        );
    }

    #[test]
    fn findings_are_saved_and_revised_while_diagnosing() {
        let (order, changes) = diagnosing(None)
            .record_diagnosis(Some(findings("Loose fan")))
            .unwrap();
        assert_eq!(order, diagnosing(Some("Loose fan")));
        assert_eq!(
            changes,
            [change("diagnostic_findings", None, Some("Loose fan"))]
        );

        let (order, changes) = order
            .record_diagnosis(Some(findings("Faulty power IC")))
            .unwrap();
        assert_eq!(order, diagnosing(Some("Faulty power IC")));
        assert_eq!(
            changes,
            [change(
                "diagnostic_findings",
                Some("Loose fan"),
                Some("Faulty power IC")
            )]
        );
    }

    #[test]
    fn diagnosis_without_findings_keeps_what_is_saved() {
        let saved = diagnosing(Some("Loose fan"));
        assert_eq!(saved.record_diagnosis(None), Ok((saved.clone(), vec![])));
        assert_eq!(
            saved.record_diagnosis(Some(findings("Loose fan"))),
            Ok((saved.clone(), vec![]))
        );
    }

    /// The decision was made on these findings and their estimate, so they are fixed after it.
    #[test]
    fn diagnosis_is_rejected_after_a_client_decision() {
        for answer in Decision::ALL {
            assert_eq!(
                decided(answer).record_diagnosis(Some(findings("Other"))),
                Err(GuardViolation::DecisionRecorded),
                "{answer:?}"
            );
            assert_eq!(
                decided(answer).record_diagnosis(None),
                Err(GuardViolation::DecisionRecorded)
            );
        }
    }

    #[test]
    fn diagnosis_cannot_reenter_from_later_states() {
        for from in [InProgress, Ready, Completed, Cancelled] {
            let order = RepairOrder {
                status: from,
                ..in_progress()
            };
            assert_eq!(
                order.record_diagnosis(Some(findings("Other"))),
                Err(GuardViolation::Transition {
                    from,
                    to: Diagnosing
                }),
                "{from}"
            );
        }
    }

    // --- Estimate (REQ-F-007) ---

    #[test]
    fn estimate_needs_saved_findings() {
        assert_eq!(
            received().record_estimate(estimate(1, 1)),
            Err(GuardViolation::FindingsMissing)
        );
        assert_eq!(
            diagnosing(None).record_estimate(estimate(1, 1)),
            Err(GuardViolation::FindingsMissing)
        );
    }

    #[test]
    fn estimate_is_itemized_and_audited_per_amount() {
        let (order, changes) = diagnosing(Some("Faulty power IC"))
            .record_estimate(estimate(10_000, 10_000))
            .unwrap();
        assert_eq!(order, estimated());
        assert_eq!(
            changes,
            [
                change("estimate.parts_cents", None, Some("10000")),
                change("estimate.labor_cents", None, Some("10000")),
            ]
        );
    }

    #[test]
    fn estimate_can_be_revised_before_the_decision() {
        let (order, changes) = estimated()
            .record_estimate(estimate(10_000, 8_000))
            .unwrap();
        assert_eq!(order.estimate, Some(estimate(10_000, 8_000)));
        assert_eq!(
            changes,
            [change("estimate.labor_cents", Some("10000"), Some("8000"))]
        );
        assert_eq!(
            estimated().record_estimate(estimate(10_000, 10_000)),
            Ok((estimated(), vec![]))
        );
    }

    /// REQ-F-007: the estimate is prepared before a client decision is recorded.
    #[test]
    fn estimate_is_rejected_after_a_client_decision() {
        for order in [
            decided(Decision::Approved),
            decided(Decision::Declined),
            in_progress(),
            ready(),
        ] {
            assert_eq!(
                order.record_estimate(estimate(1, 1)),
                Err(GuardViolation::DecisionRecorded)
            );
        }
    }

    #[test]
    fn estimate_is_rejected_on_a_closed_order() {
        let cancelled = RepairOrder {
            status: Cancelled,
            ..diagnosing(Some("Faulty power IC"))
        };
        assert_eq!(
            cancelled.record_estimate(estimate(1, 1)),
            Err(GuardViolation::Status {
                status: Cancelled,
                expected: Diagnosing
            })
        );
    }

    // --- Client decision (REQ-F-008) ---

    #[test]
    fn decision_is_recorded_with_its_time() {
        for answer in Decision::ALL {
            let (order, changes) = estimated().record_decision(decision(answer)).unwrap();
            assert_eq!(order, decided(answer));
            assert_eq!(
                changes,
                [
                    change("client_decision.decision", None, Some(answer.as_str())),
                    change(
                        "client_decision.decided_at",
                        None,
                        Some("2026-10-21T10:15:00Z")
                    ),
                ]
            );
        }
    }

    #[test]
    fn decision_time_is_kept_in_utc() {
        let local = october_21(10, 15).to_offset(UtcOffset::from_hms(2, 0, 0).unwrap());
        let (order, changes) = estimated()
            .record_decision(ClientDecision {
                decision: Decision::Approved,
                decided_at: local,
            })
            .unwrap();
        let decided_at = order.client_decision.unwrap().decided_at;
        assert_eq!(decided_at, october_21(10, 15));
        assert_eq!(decided_at.offset(), UtcOffset::UTC);
        assert_eq!(changes[1].new.as_deref(), Some("2026-10-21T10:15:00Z"));
    }

    #[test]
    fn decision_needs_an_estimate() {
        for order in [received(), diagnosing(None), diagnosing(Some("Loose fan"))] {
            assert_eq!(
                order.record_decision(decision(Decision::Approved)),
                Err(GuardViolation::EstimateMissing)
            );
        }
    }

    #[test]
    fn decision_is_recorded_once() {
        for answer in Decision::ALL {
            assert_eq!(
                decided(answer).record_decision(decision(Decision::Approved)),
                Err(GuardViolation::DecisionRecorded)
            );
        }
        assert_eq!(
            in_progress().record_decision(decision(Decision::Declined)),
            Err(GuardViolation::DecisionRecorded)
        );
    }

    #[test]
    fn decision_is_rejected_on_a_closed_order() {
        let cancelled = RepairOrder {
            status: Cancelled,
            ..estimated()
        };
        assert_eq!(
            cancelled.record_decision(decision(Decision::Approved)),
            Err(GuardViolation::Status {
                status: Cancelled,
                expected: Diagnosing
            })
        );
    }

    // --- Repair start (REQ-F-009, BDD-003, BDD-004) ---

    #[test]
    fn bdd_003_approved_order_starts_repair() {
        let (order, changes) = decided(Decision::Approved)
            .record_repair_work(work(RepairStage::InProgress))
            .unwrap();
        assert_eq!(order, in_progress());
        assert_eq!(
            changes,
            [change("status", Some("Diagnosing"), Some("InProgress"))]
        );
    }

    #[test]
    fn bdd_004_undecided_or_declined_order_cannot_start_repair() {
        for order in [
            diagnosing(None),
            diagnosing(Some("Loose fan")),
            estimated(),
            decided(Decision::Declined),
        ] {
            assert_eq!(
                order.record_repair_work(work(RepairStage::InProgress)),
                Err(GuardViolation::ApprovalMissing)
            );
        }
    }

    /// REQ-F-010: an estimate is never an incurred cost, even once repair starts.
    #[test]
    fn starting_repair_does_not_turn_the_estimate_into_charges() {
        let (order, _) = decided(Decision::Approved)
            .record_repair_work(work(RepairStage::InProgress))
            .unwrap();
        assert_eq!(order.used_parts, Cents::ZERO);
        assert_eq!(order.completed_labor, Cents::ZERO);
    }

    #[test]
    fn repair_starts_only_from_diagnosing() {
        for stage in RepairStage::ALL {
            assert_eq!(
                received().record_repair_work(work(stage)),
                Err(GuardViolation::Transition {
                    from: Received,
                    to: stage.status()
                })
            );
        }
        assert_eq!(
            decided(Decision::Approved).record_repair_work(RepairWork {
                repair_checked: Some(true),
                ..work(RepairStage::Ready)
            }),
            Err(GuardViolation::Transition {
                from: Diagnosing,
                to: Ready
            })
        );
    }
    #[test]
    fn work_requires_check_and_records_independent_charges() {
        let original = in_progress();
        assert_eq!(
            original.record_repair_work(work(RepairStage::Ready)),
            Err(GuardViolation::RepairCheckMissing)
        );
        let (updated, changes) = original
            .record_repair_work(RepairWork {
                used_parts: Some(cents(10000)),
                completed_labor: Some(cents(5)),
                repair_checked: Some(true),
                ..work(RepairStage::Ready)
            })
            .unwrap();
        assert_eq!(updated.status, Ready);
        assert_eq!(updated.used_parts, cents(10000));
        assert_eq!(updated.completed_labor, cents(5));
        assert!(updated.repair_checked);
        assert_eq!(changes.len(), 4);
        for status in [Ready, Completed, Cancelled] {
            let terminal = RepairOrder {
                status,
                ..updated.clone()
            };
            assert!(
                terminal
                    .record_repair_work(work(RepairStage::InProgress))
                    .is_err()
            );
            assert!(
                terminal
                    .record_repair_work(work(RepairStage::Ready))
                    .is_err()
            );
        }
    }
    #[test]
    fn settlement_commands_guard_every_source_state() {
        for status in OrderStatus::ALL {
            let mut order = ready();
            order.status = status;
            order.payment = Some(Payment {
                amount: cents(19000),
                paid_at: october_21(11, 0),
            });
            order.picked_up_at = Some(october_21(11, 5));
            let before = order.clone();
            assert_eq!(order.complete(LoyaltyTier::Silver).is_ok(), status == Ready);
            assert_eq!(
                order.cancel(LoyaltyTier::Silver).is_ok(),
                matches!(status, Received | Diagnosing | InProgress)
            );
            assert_eq!(
                order
                    .record_payment(Payment {
                        amount: cents(0),
                        paid_at: october_21(11, 0)
                    })
                    .is_ok(),
                status == Ready
            );
            assert_eq!(
                order.record_pickup(october_21(11, 5)).is_ok(),
                status == Ready
            );
            assert_eq!(order, before);
        }
    }

    #[test]
    fn completion_requires_exact_payment_and_pickup() {
        let mut order = ready();
        for paid in [None, Some(18999), Some(19001), Some(19000)] {
            order.payment = paid.map(|n| Payment {
                amount: cents(n),
                paid_at: october_21(11, 0),
            });
            assert!(order.complete(LoyaltyTier::Silver).is_err());
            order.picked_up_at = Some(october_21(11, 5));
            assert_eq!(
                order.complete(LoyaltyTier::Silver).is_ok(),
                paid == Some(19000)
            );
            order.picked_up_at = None;
        }
    }

    #[test]
    fn cancellation_prices_each_stage_and_saves_its_source() {
        for (order, total) in [
            (received(), 0),
            (diagnosing(None), 0),
            (estimated(), 3000),
            (decided(Decision::Approved), 3000),
            (in_progress(), 0),
            (
                RepairOrder {
                    used_parts: cents(10000),
                    completed_labor: cents(5000),
                    ..in_progress()
                },
                14000,
            ),
        ] {
            let (cancelled, _) = order.cancel(LoyaltyTier::Gold).unwrap();
            assert_eq!(cancelled.cancelled_from, Some(order.status));
            assert_eq!(cancelled.final_settlement.unwrap().total.get(), total);
            assert_eq!(
                crate::pricing::current_settlement(&cancelled, LoyaltyTier::Standard)
                    .total
                    .get(),
                total
            );
        }
    }
}
