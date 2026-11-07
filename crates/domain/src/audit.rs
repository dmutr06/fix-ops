//! The audit contract (REQ-NF-003, REQ-F-018).
//!
//! Every successful change produces one [`FieldChange`] per changed field. The store writes
//! them in the same transaction as the change itself, so a record never changes without
//! its audit entries, and a rejected change writes neither.

/// Kind of record an audit entry refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordType {
    StaffAccount,
    Client,
    Device,
    RepairOrder,
}

impl RecordType {
    pub const ALL: [Self; 4] = [
        Self::StaffAccount,
        Self::Client,
        Self::Device,
        Self::RepairOrder,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StaffAccount => "StaffAccount",
            Self::Client => "Client",
            Self::Device => "Device",
            Self::RepairOrder => "RepairOrder",
        }
    }
}

impl std::str::FromStr for RecordType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|t| t.as_str() == s)
            .ok_or_else(|| format!("unknown record type {s:?}"))
    }
}

/// One changed field with its previous and new value, rendered as text.
/// `None` means the field had no value (for example a field set on creation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldChange {
    pub field: &'static str,
    pub previous: Option<String>,
    pub new: Option<String>,
}

impl FieldChange {
    pub fn new(field: &'static str, previous: Option<String>, new: Option<String>) -> Self {
        Self {
            field,
            previous,
            new,
        }
    }

    /// A field set on a new record.
    pub fn created(field: &'static str, new: impl ToString) -> Self {
        Self::new(field, None, Some(new.to_string()))
    }

    /// A change to a secret field. Neither value is logged.
    pub fn redacted(field: &'static str) -> Self {
        Self::new(field, None, None)
    }

    /// A change from `previous` to `new`, or `None` when the value did not change.
    pub fn diff<T: PartialEq + ToString>(
        field: &'static str,
        previous: &T,
        new: &T,
    ) -> Option<Self> {
        (previous != new)
            .then(|| Self::new(field, Some(previous.to_string()), Some(new.to_string())))
    }

    /// Like [`FieldChange::diff`] for a value that may be absent.
    pub fn diff_optional<T: PartialEq + ToString>(
        field: &'static str,
        previous: Option<&T>,
        new: Option<&T>,
    ) -> Option<Self> {
        (previous != new).then(|| {
            Self::new(
                field,
                previous.map(ToString::to_string),
                new.map(ToString::to_string),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_skips_unchanged_values() {
        assert_eq!(FieldChange::diff("name", &"Bob", &"Bob"), None);
        assert_eq!(
            FieldChange::diff("name", &"Bob", &"Rob"),
            Some(FieldChange::new(
                "name",
                Some("Bob".into()),
                Some("Rob".into())
            ))
        );
    }

    #[test]
    fn created_has_no_previous_value() {
        let change = FieldChange::created("phone", "+1-555-0100");
        assert_eq!(change.previous, None);
        assert_eq!(change.new.as_deref(), Some("+1-555-0100"));
    }
}
