//! JSON extractor whose rejections use the contract's `ErrorResponse` instead of plain text.

use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use fixops_app::{AppError, ErrorKind};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};

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

/// For an optional field that may be left out but must not be `null`:
/// `#[serde(default, deserialize_with = "json::not_null")]` on an `Option<T>`.
///
/// serde calls this only when the field is present, so absence stays `None`, while an
/// explicit `null` fails to deserialize as `T` and becomes `400 VALIDATION_ERROR`. Fields the
/// contract marks `nullable` keep a plain `Option<T>`.
pub fn not_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Patch {
        #[serde(default, deserialize_with = "not_null")]
        role: Option<String>,
    }

    #[test]
    fn not_null_accepts_absent_and_values_but_rejects_null() {
        let parse = |v| serde_json::from_value::<Patch>(v);
        assert_eq!(parse(json!({})).unwrap(), Patch { role: None });
        assert_eq!(
            parse(json!({ "role": "Technician" })).unwrap(),
            Patch {
                role: Some("Technician".into())
            }
        );
        assert!(parse(json!({ "role": null })).is_err());
    }
}
