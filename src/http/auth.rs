//! The signed-in staff member of a request (REQ-F-001).

use axum::extract::FromRequestParts;
use axum::http::HeaderMap;
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::request::Parts;
use fixops_app::AppError;
use fixops_domain::staff::Actor;

use super::{ApiError, AppState};

/// Name of the browser session cookie (ADD-004).
pub const SESSION_COOKIE: &str = "fixops_session";

/// The actor behind `Authorization: Bearer <token>` or the `fixops_session` cookie.
///
/// Every protected handler takes this extractor, so a missing, unknown or expired token, or
/// the token of a disabled account, is answered with `401 UNAUTHENTICATED` before the handler
/// runs.
#[derive(Debug, Clone, Copy)]
pub struct SignedIn(pub Actor);

impl FromRequestParts<AppState> for SignedIn {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = session_token(&parts.headers).ok_or_else(AppError::unauthenticated)?;
        let actor = state.app.authenticate(token).await?;
        super::trace::record_actor(actor);
        Ok(Self(actor))
    }
}

/// The session token of a request: the bearer token, or else the session cookie.
pub fn session_token(headers: &HeaderMap) -> Option<&str> {
    bearer_token(headers).or_else(|| cookie_token(headers))
}

fn cookie_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, token)| *name == SESSION_COOKIE && !token.is_empty())
        .map(|(_, token)| token)
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn headers(value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static(value));
        headers
    }

    #[test]
    fn reads_bearer_tokens_only() {
        assert_eq!(bearer_token(&headers("Bearer abc")), Some("abc"));
        assert_eq!(bearer_token(&headers("bearer  abc ")), Some("abc"));
        assert_eq!(bearer_token(&headers("Basic abc")), None);
        assert_eq!(bearer_token(&headers("Bearer ")), None);
        assert_eq!(bearer_token(&headers("abc")), None);
        assert_eq!(bearer_token(&HeaderMap::new()), None);
    }

    #[test]
    fn reads_the_session_cookie_when_there_is_no_bearer_token() {
        let mut cookies = HeaderMap::new();
        cookies.insert(
            COOKIE,
            HeaderValue::from_static("theme=dark; fixops_session=xyz"),
        );
        assert_eq!(session_token(&cookies), Some("xyz"));
        cookies.insert(AUTHORIZATION, HeaderValue::from_static("Bearer abc"));
        assert_eq!(session_token(&cookies), Some("abc"));
        let mut empty = HeaderMap::new();
        empty.insert(COOKIE, HeaderValue::from_static("fixops_session="));
        assert_eq!(session_token(&empty), None);
    }
}
