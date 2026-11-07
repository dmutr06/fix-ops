//! Use cases of the workshop.
//!
//! Each use case lives in its own module as an `impl` block on [`App`]. A module declares the
//! store port it needs as a trait (for example `ClientStore`), and `fixops-infra` implements it.
//! This layer checks permissions, loads records, asks `fixops-domain` to apply the rules, and
//! hands the result plus its audit entries to the store in one call.

pub mod audit;
pub mod error;
pub mod staff;

pub use error::{AppError, ErrorKind, StoreError};

/// The application: use cases over a store `S`.
///
/// `S` is one adapter that implements every store port, so the HTTP layer holds a single
/// concrete `App<PgStore>` and the use cases stay testable with in-memory fakes.
#[derive(Debug, Clone)]
pub struct App<S> {
    store: S,
}

impl<S> App<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &S {
        &self.store
    }
}

/// Result of a use case.
pub type AppResult<T> = Result<T, AppError>;
