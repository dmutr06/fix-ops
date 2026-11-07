//! Test harness: drives the real router in-process against a fresh PostgreSQL database.
//!
//! Each test takes the pool that `#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]` creates,
//! so tests are isolated and run in parallel. `DATABASE_URL` must point at a server where the
//! user may create databases (see `docker-compose.yml`).

#![allow(dead_code, reason = "each test binary uses a different subset")]

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use fixops::{AppState, router};
use fixops_infra::PgStore;
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

pub struct TestApp {
    router: Router,
    pub pool: PgPool,
}

impl TestApp {
    pub fn new(pool: PgPool) -> Self {
        let router = router(AppState::new(PgStore::new(pool.clone())));
        Self { router, pool }
    }

    pub fn get(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::GET, path)
    }

    pub fn post(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::POST, path)
    }

    pub fn put(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::PUT, path)
    }

    pub fn patch(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::PATCH, path)
    }

    pub fn request(&self, method: Method, path: &str) -> TestRequest<'_> {
        TestRequest {
            app: self,
            builder: Request::builder().method(method).uri(path),
            body: Body::empty(),
        }
    }
}

pub struct TestRequest<'a> {
    app: &'a TestApp,
    builder: axum::http::request::Builder,
    body: Body,
}

impl TestRequest<'_> {
    /// Sends `Authorization: Bearer <token>`.
    #[must_use]
    pub fn token(mut self, token: &str) -> Self {
        self.builder = self
            .builder
            .header(header::AUTHORIZATION, format!("Bearer {token}"));
        self
    }

    #[must_use]
    pub fn json(mut self, body: &Value) -> Self {
        self.builder = self
            .builder
            .header(header::CONTENT_TYPE, "application/json");
        self.body = Body::from(body.to_string());
        self
    }

    /// Sends a raw body with a JSON content type, for malformed-input tests.
    #[must_use]
    pub fn raw_json(mut self, body: &'static str) -> Self {
        self.builder = self
            .builder
            .header(header::CONTENT_TYPE, "application/json");
        self.body = Body::from(body);
        self
    }

    pub async fn send(self) -> TestResponse {
        let request = self.builder.body(self.body).expect("valid request");
        let response = self
            .app
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("infallible");
        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("readable body")
            .to_bytes();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
        };
        TestResponse { status, body }
    }
}

#[derive(Debug)]
pub struct TestResponse {
    pub status: StatusCode,
    pub body: Value,
}

impl TestResponse {
    /// Asserts the status and returns the JSON body.
    #[track_caller]
    pub fn expect(self, status: StatusCode) -> Value {
        assert_eq!(
            self.status, status,
            "unexpected status, body: {:#}",
            self.body
        );
        self.body
    }

    /// Asserts an `ErrorResponse` with this status and code and returns it.
    #[track_caller]
    pub fn expect_error(self, status: StatusCode, code: &str) -> Value {
        let body = self.expect(status);
        assert_eq!(body["code"], code, "unexpected error body: {body:#}");
        assert!(body["message"].as_str().is_some_and(|m| !m.is_empty()));
        body
    }
}
