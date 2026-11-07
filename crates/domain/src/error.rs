//! The error every value constructor returns.

/// A business value was rejected. `field` is the contract field name, so the HTTP layer can
/// point the client at the offending input.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{field}: {message}")]
pub struct InvalidField {
    pub field: &'static str,
    pub message: &'static str,
}

impl InvalidField {
    pub const fn new(field: &'static str, message: &'static str) -> Self {
        Self { field, message }
    }
}

/// Trims `raw` and rejects the result when it is empty.
pub(crate) fn required_text(
    field: &'static str,
    message: &'static str,
    raw: &str,
) -> Result<String, InvalidField> {
    match raw.trim() {
        "" => Err(InvalidField::new(field, message)),
        text => Ok(text.to_owned()),
    }
}
