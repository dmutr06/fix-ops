//! Path and query extractors whose rejections use the contract's `ErrorResponse`.

use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use fixops_app::{AppError, ErrorKind};
use serde::de::DeserializeOwned;

use super::ApiError;

/// A record id from the single path parameter of a route, such as `{staffId}`. Anything but an
/// integer of at least 1 is `400 VALIDATION_ERROR` naming the parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathId(pub i64);

impl<S: Send + Sync> FromRequestParts<S> for PathId {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(params) = Path::<Vec<(String, String)>>::from_request_parts(parts, state)
            .await
            .map_err(|rejection| AppError::new(ErrorKind::Validation, rejection.body_text()))?;
        let [(name, value)] = params.as_slice() else {
            return Err(AppError::internal("route must have exactly one path parameter").into());
        };
        value
            .parse()
            .ok()
            .filter(|id| *id >= 1)
            .map(Self)
            .ok_or_else(|| invalid_id(name).into())
    }
}

/// Checks an optional id filter: present values must be at least 1.
pub fn positive_id(field: &'static str, id: Option<i64>) -> Result<Option<i64>, AppError> {
    id.map(|id| record_id(field, id)).transpose()
}

/// Checks a record id in a request body: it must be at least 1.
pub fn record_id(field: &'static str, id: i64) -> Result<i64, AppError> {
    if id >= 1 {
        Ok(id)
    } else {
        Err(invalid_id(field))
    }
}

/// Checks an optional text filter: present values must not be empty (`minLength: 1`).
pub fn non_empty(field: &'static str, text: Option<String>) -> Result<Option<String>, AppError> {
    match text {
        Some(text) if text.is_empty() => Err(AppError::validation(
            field,
            format!("{field} must not be empty"),
        )),
        _ => Ok(text),
    }
}

fn invalid_id(field: &str) -> AppError {
    AppError::new(
        ErrorKind::Validation,
        format!("{field} must be a positive integer"),
    )
    .with("field", field)
}

/// Like [`axum::extract::Query`], but a malformed query string becomes `400 VALIDATION_ERROR`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Query<T>(pub T);

impl<T, S> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Query(value)| Self(value))
            .map_err(|rejection| AppError::new(ErrorKind::Validation, rejection.body_text()).into())
    }
}
