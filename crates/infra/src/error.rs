//! Translates sqlx errors into the store errors the application layer understands.

use fixops_app::StoreError;

/// Converts a database error into a [`StoreError`], keeping the constraint name for unique and
/// foreign key violations so use cases can explain which value was rejected.
pub(crate) fn store_error(err: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(db) = &err {
        let constraint = db.constraint().unwrap_or_default().to_owned();
        if db.is_unique_violation() {
            return StoreError::Duplicate { constraint };
        }
        if db.is_foreign_key_violation() {
            return StoreError::MissingReference { constraint };
        }
    }
    StoreError::Other(Box::new(err))
}

/// A stored value that no longer parses into its domain type.
pub(crate) fn decode_error(
    err: impl Into<Box<dyn std::error::Error + Send + Sync>>,
) -> sqlx::Error {
    sqlx::Error::Decode(err.into())
}
