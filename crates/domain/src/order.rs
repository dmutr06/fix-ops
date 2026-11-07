//! Repair orders: the aggregate, its status and intake (REQ-F-005, SRS §3.1).

use std::fmt;

use time::OffsetDateTime;

use crate::audit::FieldChange;
use crate::client::Client;
use crate::device::Device;
use crate::error::{InvalidField, required_text};
use crate::id::{ClientId, DeviceId, OrderId, StaffId};
use crate::money::Cents;
use crate::pricing::Settlement;
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
    pub diagnostic_findings: Option<String>,
    pub estimate: Option<Estimate>,
    pub client_decision: Option<ClientDecision>,
    pub used_parts: Cents,
    pub completed_labor: Cents,
    pub repair_checked: bool,
    pub payment: Option<Payment>,
    pub picked_up_at: Option<OffsetDateTime>,
    /// The settlement fixed when the order reached `Completed` or `Cancelled`; `None` while open.
    pub final_settlement: Option<Settlement>,
    /// Optimistic concurrency token, increased by every saved change.
    pub version: i64,
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
            version: 1,
        }
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
}
