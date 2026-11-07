//! JSON extractor whose rejections use the contract's `ErrorResponse` instead of plain text.

use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use fixops_app::{AppError, ErrorKind};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::ApiError;

/// Like [`axum::Json`], but a malformed or invalid body becomes `400 VALIDATION_ERROR`.
///
/// Request DTOs should use `#[serde(deny_unknown_fields)]` to match `additionalProperties: false`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Json<T>(pub T);

impl<S, T> FromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(value)) => Ok(Self(value)),
            Err(rejection) => Err(rejection_error(&rejection).into()),
        }
    }
}

fn rejection_error(rejection: &JsonRejection) -> AppError {
    AppError::new(ErrorKind::Validation, rejection.body_text())
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}
