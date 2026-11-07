//! Translates sqlx errors into the store errors the application layer understands.

use fixops_app::StoreError;

/// Converts a database error into a [`StoreError`], keeping the constraint name for unique and
/// foreign key violations so use cases can explain which value was rejected.
#[expect(dead_code, reason = "used by the store ports as they are added")]
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
