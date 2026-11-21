//! The service starts against a migrated database and answers in the contract's error format.

mod common;

use axum::http::{Method, StatusCode};
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

/// Every operation of `spec/openapi.yaml` is served at its path and method.
#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn every_contract_operation_is_routed(pool: PgPool) {
    let app = TestApp::new(pool);
    let text = std::fs::read_to_string("spec/openapi.yaml").unwrap();
    let spec: serde_json::Value = serde_yaml::from_str(&text).unwrap();
    let mut operations = 0;
    for (path, item) in spec["paths"].as_object().unwrap() {
        let uri = path
            .replace("{clientId}", "1")
            .replace("{deviceId}", "1")
            .replace("{orderId}", "1")
            .replace("{staffId}", "1");
        for method in ["get", "post", "put", "patch", "delete"] {
            if item.get(method).is_none() {
                continue;
            }
            operations += 1;
            let method = Method::from_bytes(method.to_uppercase().as_bytes()).unwrap();
            let status = app.request(method.clone(), &uri).send().await.status;
            assert!(
                status != StatusCode::NOT_FOUND && status != StatusCode::METHOD_NOT_ALLOWED,
                "{method} {uri} is not routed: {status}"
            );
        }
    }
    assert_eq!(operations, 26);
}
