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
use fixops_app::App;
use fixops_domain::staff::{Password, Username};
use fixops_infra::PgStore;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

pub mod workshop;

/// Username of the Administrator created by [`TestApp::admin_token`].
pub const ADMIN: &str = "root.admin";
/// Password of every account the helpers create.
pub const PASSWORD: &str = "correct-horse-battery";

pub struct TestApp {
    router: Router,
    state: AppState,
    pub pool: PgPool,
}

impl TestApp {
    pub fn new(pool: PgPool) -> Self {
        let state = AppState::new(PgStore::new(pool.clone()));
        let router = router(state.clone());
        Self {
            router,
            state,
            pool,
        }
    }

    /// The application behind the router, for setup the API cannot do (startup bootstrap).
    pub fn app(&self) -> &App<PgStore> {
        &self.state.app
    }

    /// Creates the first Administrator the way startup does, signs in and returns the token.
    pub async fn admin_token(&self) -> String {
        let username = Username::parse(ADMIN).expect("valid username");
        let password = Password::parse(PASSWORD.to_owned()).expect("valid password");
        self.app()
            .bootstrap_admin(username, password)
            .await
            .expect("bootstrap succeeds")
            .expect("no staff account existed");
        self.sign_in(ADMIN, PASSWORD).await
    }

    /// Signs in through `POST /sessions` and returns the bearer token.
    pub async fn sign_in(&self, username: &str, password: &str) -> String {
        let body = self
            .post("/api/v1/sessions")
            .json(&json!({ "username": username, "password": password }))
            .send()
            .await
            .expect(StatusCode::CREATED);
        body["token"].as_str().expect("token string").to_owned()
    }

    /// Creates a staff account as the Administrator behind `admin_token` and returns its id.
    pub async fn create_staff(&self, admin_token: &str, username: &str, role: &str) -> i64 {
        let body = self
            .post("/api/v1/staff")
            .token(admin_token)
            .json(&json!({ "username": username, "password": PASSWORD, "role": role }))
            .send()
            .await
            .expect(StatusCode::CREATED);
        body["id"].as_i64().expect("staff id")
    }

    /// Creates a staff account, signs it in and returns its id and token.
    pub async fn staff_token(
        &self,
        admin_token: &str,
        username: &str,
        role: &str,
    ) -> (i64, String) {
        let id = self.create_staff(admin_token, username, role).await;
        (id, self.sign_in(username, PASSWORD).await)
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

    /// Sends an arbitrary header, for malformed-credential tests.
    #[must_use]
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.builder = self.builder.header(name, value);
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
