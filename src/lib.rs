//! HTTP layer of the FixOps service: routing, JSON DTOs, authentication and error mapping.
//!
//! Handlers stay thin. They parse the request into domain values, call one use case on
//! [`fixops_app::App`], and render the result. Business rules live in `fixops-domain`.

pub mod config;
pub mod http;

pub use http::{AppState, router};
