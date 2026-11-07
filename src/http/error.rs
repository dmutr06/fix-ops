//! Renders every failure as the contract's `ErrorResponse` (ADD-002 §5).

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use fixops_app::{AppError, ErrorKind};
use serde::Serialize;

/// An [`AppError`] on its way to the client.
#[derive(Debug)]
pub struct ApiError(pub AppError);

impl<E: Into<AppError>> From<E> for ApiError {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[derive(Serialize)]
struct ErrorResponse<'a> {
    code: &'static str,
    message: &'a str,
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    details: serde_json::Map<String, serde_json::Value>,
}

const fn status_and_code(kind: ErrorKind) -> (StatusCode, &'static str) {
    match kind {
        ErrorKind::Validation => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
        ErrorKind::Unauthenticated => (StatusCode::UNAUTHORIZED, "UNAUTHENTICATED"),
        ErrorKind::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN"),
        ErrorKind::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
        ErrorKind::Conflict => (StatusCode::CONFLICT, "CONFLICT"),
        ErrorKind::Guard => (StatusCode::CONFLICT, "GUARD_VIOLATION"),
        // The contract has no 5xx error code; INTERNAL is outside the documented enum on purpose.
        ErrorKind::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL"),
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let err = self.0;
        let (status, code) = status_and_code(err.kind);
        if err.kind == ErrorKind::Internal {
            tracing::error!(error = ?err, "request failed");
        }

        let details = err
            .details
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.into()))
            .collect();
        let body = ErrorResponse {
            code,
            message: &err.message,
            details,
        };
        (status, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_maps_to_the_contract() {
        assert_eq!(
            status_and_code(ErrorKind::Validation).0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status_and_code(ErrorKind::Unauthenticated).0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status_and_code(ErrorKind::Forbidden).0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            status_and_code(ErrorKind::NotFound).0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            status_and_code(ErrorKind::Conflict),
            (StatusCode::CONFLICT, "CONFLICT")
        );
        assert_eq!(
            status_and_code(ErrorKind::Guard),
            (StatusCode::CONFLICT, "GUARD_VIOLATION")
        );
    }
}
