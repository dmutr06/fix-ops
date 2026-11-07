//! Business rules of the workshop: records, roles, lifecycle and pricing.
//!
//! This crate is pure. It does no I/O and knows nothing about HTTP or SQL,
//! so every rule here is tested with plain unit tests.

pub mod audit;
pub mod error;
pub mod id;
pub mod staff;
