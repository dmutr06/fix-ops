mod common;

use axum::http::StatusCode;
use common::TestApp;
use common::workshop::{get_order, in_progress_order, put, repair, set_tier, snapshot, staff};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn repair_check_and_current_tier_apply_to_order_views(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = in_progress_order(&app, &s, "Silver").await;
    let recorded = repair(
        &app,
        &s,
        &order,
        &json!({"status":"InProgress", "used_parts_cents":10000, "completed_labor_cents":5}),
    )
    .await;
    assert_eq!(recorded["labor_discount_cents"], 1);
    assert_eq!(recorded["settlement_cents"], 10004);
    let before = snapshot(&app, &s.admin, &order).await;
    put(&app, &s.tech, &order, "repair", &json!({"status":"Ready"}))
        .await
        .expect_error(StatusCode::CONFLICT, "GUARD_VIOLATION");
    assert_eq!(snapshot(&app, &s.admin, &order).await, before);
    set_tier(&app, &s, order.client, "Gold").await;
    let ready = repair(
        &app,
        &s,
        &order,
        &json!({"status":"Ready", "repair_checked":true, "completed_labor_cents":10000}),
    )
    .await;
    assert_eq!(ready["settlement_cents"], 18000);
    assert_eq!(get_order(&app, &s.recep, &order).await, ready);
    let listed = app
        .get("/api/v1/orders")
        .token(&s.recep)
        .send()
        .await
        .expect(StatusCode::OK);
    assert_eq!(listed[0], ready);
    let before = snapshot(&app, &s.admin, &order).await;
    put(
        &app,
        &s.admin,
        &order,
        "repair",
        &json!({"status":"InProgress"}),
    )
    .await
    .expect_error(StatusCode::CONFLICT, "GUARD_VIOLATION");
    assert_eq!(snapshot(&app, &s.admin, &order).await, before);
}
