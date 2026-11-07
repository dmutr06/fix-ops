//! Staff accounts, roles, the permission matrix and the signed-in actor.

use std::fmt;

use crate::audit::FieldChange;
use crate::error::InvalidField;
use crate::id::StaffId;

/// Staff role from SRS §2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Receptionist,
    Technician,
    Administrator,
}

impl Role {
    pub const ALL: [Self; 3] = [Self::Receptionist, Self::Technician, Self::Administrator];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Receptionist => "Receptionist",
            Self::Technician => "Technician",
            Self::Administrator => "Administrator",
        }
    }

    /// Whether this role may perform `action`. Administrator gets its Receptionist and
    /// Technician actions from the same matrix, not from a bypass (ADD-001).
    pub fn can(self, action: Action) -> bool {
        action.roles().contains(&self)
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Role {
    type Err = UnknownRole;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|role| role.as_str() == s)
            .ok_or_else(|| UnknownRole(s.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown role {0:?}")]
pub struct UnknownRole(pub String);

/// A protected operation of the contract. Each variant is one `operationId` that has `x-roles`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    ListStaffAccounts,
    CreateStaffAccount,
    UpdateStaffAccount,
    ListClients,
    CreateClient,
    GetClient,
    UpdateClient,
    SetClientLoyaltyTier,
    ListDevices,
    CreateDevice,
    GetDevice,
    UpdateDevice,
    ListRepairOrders,
    CreateRepairOrder,
    GetRepairOrder,
    CorrectRepairOrder,
    RecordDiagnosis,
    RecordEstimate,
    RecordClientDecision,
    UpdateRepairWork,
    RecordPayment,
    RecordPickup,
    CompleteRepairOrder,
    CancelRepairOrder,
    ListAuditLogs,
}

impl Action {
    pub const ALL: [Self; 25] = [
        Self::ListStaffAccounts,
        Self::CreateStaffAccount,
        Self::UpdateStaffAccount,
        Self::ListClients,
        Self::CreateClient,
        Self::GetClient,
        Self::UpdateClient,
        Self::SetClientLoyaltyTier,
        Self::ListDevices,
        Self::CreateDevice,
        Self::GetDevice,
        Self::UpdateDevice,
        Self::ListRepairOrders,
        Self::CreateRepairOrder,
        Self::GetRepairOrder,
        Self::CorrectRepairOrder,
        Self::RecordDiagnosis,
        Self::RecordEstimate,
        Self::RecordClientDecision,
        Self::UpdateRepairWork,
        Self::RecordPayment,
        Self::RecordPickup,
        Self::CompleteRepairOrder,
        Self::CancelRepairOrder,
        Self::ListAuditLogs,
    ];

    /// The roles that may perform this action: `x-roles` of its operation in the contract.
    pub const fn roles(self) -> &'static [Role] {
        const ANY_STAFF: &[Role] = &[Role::Receptionist, Role::Technician, Role::Administrator];
        const FRONT_DESK: &[Role] = &[Role::Receptionist, Role::Administrator];
        const BENCH: &[Role] = &[Role::Technician, Role::Administrator];
        const ADMINISTRATOR: &[Role] = &[Role::Administrator];

        match self {
            Self::ListClients
            | Self::GetClient
            | Self::ListDevices
            | Self::GetDevice
            | Self::ListRepairOrders
            | Self::GetRepairOrder
            | Self::ListAuditLogs => ANY_STAFF,
            Self::CreateClient
            | Self::UpdateClient
            | Self::CreateDevice
            | Self::UpdateDevice
            | Self::CreateRepairOrder
            | Self::RecordClientDecision
            | Self::RecordPayment
            | Self::RecordPickup
            | Self::CompleteRepairOrder
            | Self::CancelRepairOrder => FRONT_DESK,
            Self::RecordDiagnosis | Self::RecordEstimate | Self::UpdateRepairWork => BENCH,
            Self::ListStaffAccounts
            | Self::CreateStaffAccount
            | Self::UpdateStaffAccount
            | Self::SetClientLoyaltyTier
            | Self::CorrectRepairOrder => ADMINISTRATOR,
        }
    }

    /// The contract `operationId` of this action.
    pub const fn operation_id(self) -> &'static str {
        match self {
            Self::ListStaffAccounts => "listStaffAccounts",
            Self::CreateStaffAccount => "createStaffAccount",
            Self::UpdateStaffAccount => "updateStaffAccount",
            Self::ListClients => "listClients",
            Self::CreateClient => "createClient",
            Self::GetClient => "getClient",
            Self::UpdateClient => "updateClient",
            Self::SetClientLoyaltyTier => "setClientLoyaltyTier",
            Self::ListDevices => "listDevices",
            Self::CreateDevice => "createDevice",
            Self::GetDevice => "getDevice",
            Self::UpdateDevice => "updateDevice",
            Self::ListRepairOrders => "listRepairOrders",
            Self::CreateRepairOrder => "createRepairOrder",
            Self::GetRepairOrder => "getRepairOrder",
            Self::CorrectRepairOrder => "correctRepairOrder",
            Self::RecordDiagnosis => "recordDiagnosis",
            Self::RecordEstimate => "recordEstimate",
            Self::RecordClientDecision => "recordClientDecision",
            Self::UpdateRepairWork => "updateRepairWork",
            Self::RecordPayment => "recordPayment",
            Self::RecordPickup => "recordPickup",
            Self::CompleteRepairOrder => "completeRepairOrder",
            Self::CancelRepairOrder => "cancelRepairOrder",
            Self::ListAuditLogs => "listAuditLogs",
        }
    }
}

/// A signed-in staff member tried an action their role does not allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{role} may not perform {action:?}")]
pub struct Denied {
    pub role: Role,
    pub action: Action,
}

/// The signed-in staff member performing an action. Every change is attributed to one actor (REQ-NF-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor {
    pub id: StaffId,
    pub role: Role,
}

impl Actor {
    /// Succeeds when the actor's role may perform `action`.
    pub fn require(self, action: Action) -> Result<(), Denied> {
        if self.role.can(action) {
            Ok(())
        } else {
            Err(Denied {
                role: self.role,
                action,
            })
        }
    }
}

/// A staff sign-in name: trimmed and non-empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Username(String);

impl Username {
    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        match raw.trim() {
            "" => Err(InvalidField::new("username", "username is required")),
            name => Ok(Self(name.to_owned())),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A new account password. Its `Debug` output never shows the secret.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

impl Password {
    /// Fewest characters a new password may have.
    pub const MIN_CHARS: usize = 8;

    pub fn parse(raw: String) -> Result<Self, InvalidField> {
        if raw.chars().count() < Self::MIN_CHARS {
            return Err(InvalidField::new(
                "password",
                "password must be at least 8 characters",
            ));
        }
        Ok(Self(raw))
    }

    /// The secret, for hashing only.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password(***)")
    }
}

/// A staff account as staff see it. The password hash never leaves the store adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffAccount {
    pub id: StaffId,
    pub username: Username,
    pub role: Role,
    pub enabled: bool,
}

impl StaffAccount {
    /// Applies a role and/or enabled change and returns the updated account with its audit
    /// changes. Values equal to the current ones produce no change.
    #[must_use]
    pub fn update(&self, role: Option<Role>, enabled: Option<bool>) -> (Self, Vec<FieldChange>) {
        let updated = Self {
            role: role.unwrap_or(self.role),
            enabled: enabled.unwrap_or(self.enabled),
            ..self.clone()
        };
        let changes = [
            FieldChange::diff("role", &self.role, &updated.role),
            FieldChange::diff("enabled", &self.enabled, &updated.enabled),
        ]
        .into_iter()
        .flatten()
        .collect();
        (updated, changes)
    }
}

/// A staff account about to be created. New accounts start enabled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewStaffAccount {
    pub username: Username,
    pub password: Password,
    pub role: Role,
}

impl NewStaffAccount {
    /// Audit changes for the creation: every set field, with the password redacted.
    pub fn changes(&self) -> Vec<FieldChange> {
        vec![
            FieldChange::created("username", &self.username),
            FieldChange::created("role", self.role),
            FieldChange::created("enabled", true),
            FieldChange::redacted("password"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(role: Role, enabled: bool) -> StaffAccount {
        StaffAccount {
            id: StaffId(7),
            username: Username::parse("max.tech").unwrap(),
            role,
            enabled,
        }
    }

    #[test]
    fn role_round_trips_through_its_name() {
        for role in Role::ALL {
            assert_eq!(role.as_str().parse::<Role>(), Ok(role));
        }
        assert!("Manager".parse::<Role>().is_err());
    }

    #[test]
    fn only_administrator_manages_staff_tiers_and_corrections() {
        for action in [
            Action::ListStaffAccounts,
            Action::CreateStaffAccount,
            Action::UpdateStaffAccount,
            Action::SetClientLoyaltyTier,
            Action::CorrectRepairOrder,
        ] {
            assert!(Role::Administrator.can(action), "{action:?}");
            assert!(!Role::Receptionist.can(action), "{action:?}");
            assert!(!Role::Technician.can(action), "{action:?}");
        }
    }

    #[test]
    fn every_role_reads_records_and_audit() {
        for action in [
            Action::ListClients,
            Action::GetClient,
            Action::ListDevices,
            Action::GetDevice,
            Action::ListRepairOrders,
            Action::GetRepairOrder,
            Action::ListAuditLogs,
        ] {
            for role in Role::ALL {
                assert!(role.can(action), "{role} {action:?}");
            }
        }
    }

    #[test]
    fn technician_cannot_cancel_or_take_front_desk_actions() {
        for action in [
            Action::CancelRepairOrder,
            Action::CreateClient,
            Action::CreateRepairOrder,
            Action::RecordClientDecision,
            Action::RecordPayment,
            Action::RecordPickup,
            Action::CompleteRepairOrder,
        ] {
            assert!(!Role::Technician.can(action), "{action:?}");
            assert!(Role::Receptionist.can(action), "{action:?}");
        }
    }

    #[test]
    fn receptionist_cannot_do_bench_work() {
        for action in [
            Action::RecordDiagnosis,
            Action::RecordEstimate,
            Action::UpdateRepairWork,
        ] {
            assert!(!Role::Receptionist.can(action), "{action:?}");
            assert!(Role::Technician.can(action), "{action:?}");
        }
    }

    #[test]
    fn administrator_inherits_every_staff_action() {
        for action in Action::ALL {
            assert!(Role::Administrator.can(action), "{action:?}");
        }
    }

    #[test]
    fn operation_ids_are_unique_camel_case() {
        let ids: std::collections::HashSet<_> =
            Action::ALL.iter().map(|a| a.operation_id()).collect();
        assert_eq!(ids.len(), Action::ALL.len());
        assert_eq!(
            Action::ListStaffAccounts.operation_id(),
            "listStaffAccounts"
        );
        assert_eq!(Action::ListAuditLogs.operation_id(), "listAuditLogs");
    }

    #[test]
    fn require_reports_the_denied_action() {
        let tech = Actor {
            id: StaffId(1),
            role: Role::Technician,
        };
        assert_eq!(tech.require(Action::RecordDiagnosis), Ok(()));
        assert_eq!(
            tech.require(Action::CancelRepairOrder),
            Err(Denied {
                role: Role::Technician,
                action: Action::CancelRepairOrder
            })
        );
    }

    #[test]
    fn username_is_trimmed_and_required() {
        assert_eq!(Username::parse("  anna  ").unwrap().as_str(), "anna");
        for blank in ["", "   ", "\t\n"] {
            assert_eq!(Username::parse(blank).unwrap_err().field, "username");
        }
    }

    #[test]
    fn password_needs_eight_characters() {
        assert_eq!(
            Password::parse("1234567".into()).unwrap_err().field,
            "password"
        );
        assert!(Password::parse("12345678".into()).is_ok());
        // Characters, not bytes: seven two-byte letters are still too short.
        assert!(Password::parse("ééééééé".into()).is_err());
    }

    #[test]
    fn password_debug_hides_the_secret() {
        let password = Password::parse("hunter2-hunter2".into()).unwrap();
        assert!(!format!("{password:?}").contains("hunter2"));
    }

    #[test]
    fn creation_audits_every_field_and_redacts_the_password() {
        let new = NewStaffAccount {
            username: Username::parse("max.tech").unwrap(),
            password: Password::parse("initial-password-123".into()).unwrap(),
            role: Role::Technician,
        };
        assert_eq!(
            new.changes(),
            vec![
                FieldChange::created("username", "max.tech"),
                FieldChange::created("role", "Technician"),
                FieldChange::created("enabled", true),
                FieldChange::new("password", None, None),
            ]
        );
    }

    #[test]
    fn update_records_only_changed_fields() {
        let tech = account(Role::Technician, true);

        let (same, changes) = tech.update(Some(Role::Technician), Some(true));
        assert_eq!(same, tech);
        assert_eq!(changes, []);

        let (updated, changes) = tech.update(Some(Role::Receptionist), Some(false));
        assert_eq!(updated.role, Role::Receptionist);
        assert!(!updated.enabled);
        assert_eq!(
            changes,
            vec![
                FieldChange::new(
                    "role",
                    Some("Technician".into()),
                    Some("Receptionist".into())
                ),
                FieldChange::new("enabled", Some("true".into()), Some("false".into())),
            ]
        );
    }

    #[test]
    fn update_keeps_omitted_fields() {
        let tech = account(Role::Technician, true);
        let (updated, changes) = tech.update(None, Some(false));
        assert_eq!(updated.role, Role::Technician);
        assert_eq!(changes.len(), 1);
    }
}
