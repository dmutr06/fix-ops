//! Errors a use case can return. The HTTP layer maps [`ErrorKind`] to a status code and an
//! `ErrorResponse` code (ADD-002 §5); this layer only says what went wrong.

use std::borrow::Cow;

use fixops_domain::error::InvalidField;
use fixops_domain::staff::Denied;

/// What kind of failure happened. Every kind except [`ErrorKind::Internal`] leaves stored
/// records unchanged and is the caller's fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Missing or invalid input.
    Validation,
    /// No valid staff session.
    Unauthenticated,
    /// Signed in, but the role or account may not do this.
    Forbidden,
    /// The record does not exist.
    NotFound,
    /// A unique value is already taken.
    Conflict,
    /// A lifecycle, approval, check, payment or pickup rule blocks the action.
    Guard,
    /// Something failed that the caller cannot fix.
    Internal,
}

/// A failed use case: its kind, a message for staff, and optional key/value details.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: Cow<'static, str>,
    pub details: Vec<(&'static str, String)>,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: Vec::new(),
            source: None,
        }
    }

    /// Invalid input in `field`.
    pub fn validation(field: &'static str, message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Validation, message).with("field", field)
    }

    pub fn unauthenticated() -> Self {
        Self::new(ErrorKind::Unauthenticated, "Staff sign-in is required")
    }

    pub fn forbidden(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Forbidden, message)
    }

    pub fn not_found() -> Self {
        Self::new(ErrorKind::NotFound, "Requested record does not exist")
    }

    pub fn conflict(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Conflict, message)
    }

    pub fn guard(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Guard, message)
    }

    pub fn internal(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self {
            source: Some(source.into()),
            ..Self::new(ErrorKind::Internal, "Internal server error")
        }
    }

    /// Adds a detail shown to the client, such as the offending field or value.
    #[must_use]
    pub fn with(mut self, key: &'static str, value: impl ToString) -> Self {
        self.details.push((key, value.to_string()));
        self
    }
}

/// Failure reported by a store adapter.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// A unique constraint rejected the write. `constraint` is the constraint name.
    #[error("unique constraint {constraint} violated")]
    Duplicate { constraint: String },
    /// A referenced record does not exist. `constraint` is the foreign key name.
    #[error("foreign key {constraint} violated")]
    MissingReference { constraint: String },
    /// The record changed since it was read; the write was not applied.
    #[error("record was changed concurrently")]
    Stale,
    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl From<InvalidField> for AppError {
    fn from(err: InvalidField) -> Self {
        Self::validation(err.field, err.message)
    }
}

impl From<Denied> for AppError {
    fn from(_: Denied) -> Self {
        Self::forbidden("Role is not authorized to perform this action")
    }
}

impl From<StoreError> for AppError {
    /// Fallback mapping. Use cases that expect a duplicate or missing reference map it themselves
    /// to a precise message.
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::Duplicate { constraint } => {
                Self::conflict("Value already exists").with("constraint", constraint)
            }
            StoreError::MissingReference { constraint } => {
                Self::new(ErrorKind::NotFound, "Referenced record does not exist")
                    .with("constraint", constraint)
            }
            StoreError::Stale => {
                Self::guard("Record was changed by another request; reload and retry")
            }
            StoreError::Other(source) => Self::internal(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_names_the_field() {
        let err = AppError::validation("phone", "phone is required");
        assert_eq!(err.kind, ErrorKind::Validation);
        assert_eq!(err.details, vec![("field", "phone".to_owned())]);
    }

    #[test]
    fn domain_errors_map_to_validation_and_forbidden() {
        let invalid = AppError::from(InvalidField::new("username", "username is required"));
        assert_eq!(invalid.kind, ErrorKind::Validation);
        assert_eq!(invalid.details, vec![("field", "username".to_owned())]);

        let denied = AppError::from(Denied {
            role: fixops_domain::staff::Role::Technician,
            action: fixops_domain::staff::Action::CancelRepairOrder,
        });
        assert_eq!(denied.kind, ErrorKind::Forbidden);
    }

    #[test]
    fn store_errors_map_to_caller_facing_kinds() {
        let dup = StoreError::Duplicate {
            constraint: "x".into(),
        };
        assert_eq!(AppError::from(dup).kind, ErrorKind::Conflict);
        assert_eq!(AppError::from(StoreError::Stale).kind, ErrorKind::Guard);
        let other = StoreError::Other("boom".into());
        assert_eq!(AppError::from(other).kind, ErrorKind::Internal);
    }
}
