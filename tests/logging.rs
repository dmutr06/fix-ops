//! Every request and every action outcome is written to the operational log, without secrets.

mod common;

use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};

use axum::http::StatusCode;
use common::{PASSWORD, TestApp};
use serde_json::{Value, json};
use sqlx::PgPool;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;

/// Collects log output in memory.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Captured {
    fn lines(&self) -> Vec<Value> {
        let bytes = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        String::from_utf8(bytes)
            .expect("utf-8 log")
            .lines()
            .map(|line| serde_json::from_str(line).expect("each line is JSON"))
            .collect()
    }

    fn raw(&self) -> String {
        let bytes = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        String::from_utf8(bytes).expect("utf-8 log")
    }
}

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn capture() -> (Captured, tracing::subscriber::DefaultGuard) {
    let captured = Captured::default();
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(fixops::telemetry::json_layer(captured.clone()));
    (captured, tracing::subscriber::set_default(subscriber))
}

/// The closing line of the request to `method path`.
fn request_line<'a>(lines: &'a [Value], method: &str, path: &str) -> &'a Value {
    lines
        .iter()
        .find(|l| {
            l["fields"]["message"] == "request finished"
                && l["span"]["method"] == method
                && l["span"]["path"] == path
        })
        .unwrap_or_else(|| panic!("no request line for {method} {path}: {lines:#?}"))
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn every_request_is_logged_with_status_latency_and_actor(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let (captured, _guard) = capture();

    app.get("/api/v1/staff")
        .token(&admin)
        .send()
        .await
        .expect(StatusCode::OK);
    app.get("/api/v1/staff")
        .send()
        .await
        .expect(StatusCode::UNAUTHORIZED);

    let lines = captured.lines();
    let ok = request_line(&lines, "GET", "/api/v1/staff");
    assert_eq!(ok["level"], "INFO");
    assert_eq!(ok["fields"]["status"], 200);
    assert!(ok["fields"]["latency_ms"].is_u64());
    assert!(
        ok["span"]["request_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    assert!(ok["span"]["actor_id"].is_u64());
    assert_eq!(ok["span"]["action"], "listStaffAccounts");

    let denied = lines
        .iter()
        .filter(|l| l["fields"]["message"] == "request finished")
        .find(|l| l["fields"]["status"] == 401)
        .expect("unauthenticated request is logged");
    assert_eq!(denied["level"], "WARN");
    assert!(denied["span"].get("actor_id").is_none());
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn saved_changes_are_logged_with_record_and_fields(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let (captured, _guard) = capture();

    let id = app.create_staff(&admin, "max.tech", "Technician").await;

    let lines = captured.lines();
    let change = lines
        .iter()
        .find(|l| l["fields"]["message"] == "record changed")
        .unwrap_or_else(|| panic!("no change line: {lines:#?}"));
    assert_eq!(change["level"], "INFO");
    assert_eq!(change["fields"]["record_type"], "StaffAccount");
    assert_eq!(change["fields"]["record_id"], id);
    assert!(change["fields"]["actor_id"].is_u64());
    let fields = change["fields"]["fields"].as_str().expect("field list");
    assert!(fields.contains("username") && fields.contains("role"));
    assert_eq!(change["span"]["action"], "createStaffAccount");
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn denied_and_rejected_actions_are_logged(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let (_, tech) = app.staff_token(&admin, "max.tech", "Technician").await;
    let (captured, _guard) = capture();

    app.get("/api/v1/staff")
        .token(&tech)
        .send()
        .await
        .expect(StatusCode::FORBIDDEN);
    app.post("/api/v1/staff")
        .token(&admin)
        .json(&json!({ "username": "max.tech", "password": PASSWORD, "role": "Technician" }))
        .send()
        .await
        .expect(StatusCode::CONFLICT);

    let lines = captured.lines();
    let denial = lines
        .iter()
        .find(|l| l["fields"]["message"] == "action denied")
        .unwrap_or_else(|| panic!("no denial line: {lines:#?}"));
    assert_eq!(denial["level"], "WARN");
    assert_eq!(denial["fields"]["action"], "listStaffAccounts");
    assert_eq!(denial["fields"]["role"], "Technician");

    let rejection = lines
        .iter()
        .find(|l| l["fields"]["message"] == "request rejected" && l["fields"]["code"] == "CONFLICT")
        .unwrap_or_else(|| panic!("no rejection line: {lines:#?}"));
    assert_eq!(rejection["level"], "WARN");
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn passwords_and_tokens_never_reach_the_log(pool: PgPool) {
    let app = TestApp::new(pool);
    let (captured, _guard) = capture();
    let admin = app.admin_token().await;
    app.create_staff(&admin, "max.tech", "Technician").await;
    app.sign_in("max.tech", PASSWORD).await;

    let log = captured.raw();
    assert_ne!(log, "");
    assert!(!log.contains(PASSWORD), "password in log");
    assert!(!log.contains(&admin), "token in log");
    assert!(!log.contains("$argon2"), "password hash in log");
}
