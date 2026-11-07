//! The service starts against a migrated database and answers in the contract's error format.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use sqlx::PgPool;

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn health_check_answers(pool: PgPool) {
    let app = TestApp::new(pool);
    assert_eq!(app.get("/health").send().await.expect(StatusCode::OK), "ok");
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn unknown_route_returns_error_response(pool: PgPool) {
    let app = TestApp::new(pool);
    app.get("/api/v1/nowhere")
        .send()
        .await
        .expect_error(StatusCode::NOT_FOUND, "NOT_FOUND");
}
