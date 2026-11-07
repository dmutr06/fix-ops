mod common;

use axum::http::StatusCode;
use common::workshop::*;
use common::{TestApp, TestResponse};
use serde_json::{Value, json};
use sqlx::PgPool;

const AT: &str = "2026-10-23T15:00:00Z";

async fn patch(app: &TestApp, token: &str, order: &Order, body: Value) -> TestResponse {
    app.patch(&format!("/api/v1/orders/{}", order.id))
        .token(token)
        .json(&body)
        .send()
        .await
}

async fn reject(
    app: &TestApp,
    s: &Staff,
    order: &Order,
    body: Value,
    status: StatusCode,
    code: &str,
) {
    let before = snapshot(app, &s.admin, order).await;
    patch(app, &s.admin, order, body)
        .await
        .expect_error(status, code);
    assert_eq!(snapshot(app, &s.admin, order).await, before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_016_correct_labor_preserves_history_and_audits_before_after(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = in_progress_order(&app, &s, "Silver").await;
    repair(
        &app,
        &s,
        &order,
        &json!({"status":"InProgress", "completed_labor_cents":10000}),
    )
    .await;
    let before = order_audit(&app, &s.admin, &order).await;
    let corrected = patch(
        &app,
        &s.admin,
        &order,
        json!({"completed_labor_cents":8000}),
    )
    .await
    .expect(StatusCode::OK);
    assert_eq!(corrected["completed_labor_cents"], 8000);
    assert_eq!(corrected["settlement_cents"], 7200);
    assert_eq!(get_order(&app, &s.admin, &order).await, corrected);
    let after = order_audit(&app, &s.admin, &order).await;
    assert_eq!(&after[..before.len()], before);
    assert_eq!(
        changes(&after[before.len()..]),
        vec![change(
            "completed_labor_cents",
            json!("10000"),
            json!("8000"),
            s.admin_id
        )]
    );
    assert!(after.last().unwrap()["recorded_at"].as_str().is_some());
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn correction_sets_and_clears_nullable_fields_and_validates_joint_state(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    let other = received_order(&app, &s, "Gold").await;
    let (tech, _) = app.staff_token(&s.admin, "other.tech", "Technician").await;
    let body = json!({"status":"Diagnosing", "ticket_number":"CORRECTED", "client_id":other.client, "device_id":other.device, "assigned_technician_id":tech,
        "diagnostic_findings":"Correct findings", "estimate":{"parts_cents":50,"labor_cents":100}, "client_decision":{"decision":"Declined", "decided_at":AT}});
    let result = patch(&app, &s.admin, &order, body.clone())
        .await
        .expect(StatusCode::OK);
    for (key, value) in body.as_object().unwrap() {
        assert_eq!(result[key], *value);
    }
    reject(
        &app,
        &s,
        &order,
        json!({"diagnostic_findings":null}),
        StatusCode::CONFLICT,
        "GUARD_VIOLATION",
    )
    .await;
    let cleared = patch(
        &app,
        &s.admin,
        &order,
        json!({"diagnostic_findings":null,"estimate":null,"client_decision":null}),
    )
    .await
    .expect(StatusCode::OK);
    assert!(cleared["diagnostic_findings"].is_null());
    assert!(cleared["estimate"].is_null());
    assert!(cleared["client_decision"].is_null());
    assert_eq!(cleared["ticket_number"], "CORRECTED");
    let audit = order_audit(&app, &s.admin, &order).await;
    for field in [
        "diagnostic_findings",
        "estimate.parts_cents",
        "estimate.labor_cents",
        "client_decision.decision",
        "client_decision.decided_at",
    ] {
        assert!(audit.iter().any(|e| e["field"] == field
            && e["new_value"].is_null()
            && !e["previous_value"].is_null()));
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn completed_metadata_correction_keeps_settlement_after_tier_change(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = in_progress_order(&app, &s, "Silver").await;
    repair(&app, &s, &order, &json!({"status":"Ready", "used_parts_cents":10000, "completed_labor_cents":10000, "repair_checked":true})).await;
    patch(&app, &s.admin, &order, json!({"status":"Completed", "payment":{"amount_cents":19000,"paid_at":AT}, "pickup":{"picked_up_at":AT}})).await.expect(StatusCode::OK);
    set_tier(&app, &s, order.client, "Gold").await;
    let before = order_audit(&app, &s.admin, &order).await;
    let corrected = patch(&app, &s.admin, &order, json!({"ticket_number":"COMPLETED-FIXED", "used_parts_cents":10000, "completed_labor_cents":10000})).await.expect(StatusCode::OK);
    assert_eq!(corrected["settlement_cents"], 19000);
    assert_eq!(corrected["labor_discount_cents"], 1000);
    assert_eq!(get_order(&app, &s.admin, &order).await, corrected);
    let after = order_audit(&app, &s.admin, &order).await;
    assert_eq!(&after[..before.len()], before);
    assert_eq!(
        changes(&after[before.len()..]),
        vec![change(
            "ticket_number",
            json!(order.ticket),
            json!("COMPLETED-FIXED"),
            s.admin_id
        )]
    );
    reject(
        &app,
        &s,
        &order,
        json!({"payment":{"amount_cents":18000,"paid_at":AT}}),
        StatusCode::CONFLICT,
        "GUARD_VIOLATION",
    )
    .await;
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn cancelled_metadata_correction_keeps_settlement_after_tier_change(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = in_progress_order(&app, &s, "Silver").await;
    repair(
        &app,
        &s,
        &order,
        &json!({"status":"InProgress", "used_parts_cents":10000, "completed_labor_cents":10000}),
    )
    .await;
    app.post(&order.path("cancel"))
        .token(&s.admin)
        .send()
        .await
        .expect(StatusCode::OK);
    set_tier(&app, &s, order.client, "Gold").await;
    let before = order_audit(&app, &s.admin, &order).await;
    let corrected = patch(
        &app,
        &s.admin,
        &order,
        json!({"ticket_number":"CANCELLED-FIXED", "completed_labor_cents":10000}),
    )
    .await
    .expect(StatusCode::OK);
    assert_eq!(corrected["settlement_cents"], 19000);
    assert_eq!(corrected["labor_discount_cents"], 1000);
    assert_eq!(get_order(&app, &s.admin, &order).await, corrected);
    let after = order_audit(&app, &s.admin, &order).await;
    assert_eq!(&after[..before.len()], before);
    assert_eq!(
        changes(&after[before.len()..]),
        vec![change(
            "ticket_number",
            json!(order.ticket),
            json!("CANCELLED-FIXED"),
            s.admin_id
        )]
    );
    let financial = patch(
        &app,
        &s.admin,
        &order,
        json!({"completed_labor_cents":8000}),
    )
    .await
    .expect(StatusCode::OK);
    assert_eq!(financial["settlement_cents"], 16400);
    assert_eq!(financial["labor_discount_cents"], 1600);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn terminal_corrections_recalculate_exact_payment_without_reopening(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = in_progress_order(&app, &s, "Silver").await;
    repair(&app, &s, &order, &json!({"status":"Ready", "used_parts_cents":10000, "completed_labor_cents":10000,"repair_checked":true})).await;
    patch(
        &app,
        &s.admin,
        &order,
        json!({"payment":{"amount_cents":19000,"paid_at":AT},"pickup":{"picked_up_at":AT}}),
    )
    .await
    .expect(StatusCode::OK);
    // Both nullable subresources can be cleared while Ready.
    let cleared = patch(
        &app,
        &s.admin,
        &order,
        json!({"payment":null,"pickup":null}),
    )
    .await
    .expect(StatusCode::OK);
    assert!(cleared["payment"].is_null() && cleared["pickup"].is_null());
    patch(&app, &s.admin, &order, json!({"status":"Completed", "payment":{"amount_cents":19000,"paid_at":AT},"pickup":{"picked_up_at":AT}})).await.expect(StatusCode::OK);
    set_tier(&app, &s, order.client, "Gold").await;
    for body in [
        json!({"status":"Ready"}),
        json!({"status":"Cancelled"}),
        json!({"completed_labor_cents":8000}),
        json!({"client_decision":null}),
        json!({"client_decision":{"decision":"Declined","decided_at":AT}}),
        json!({"repair_checked":false}),
        json!({"payment":null}),
        json!({"pickup":null}),
        json!({"payment":{"amount_cents":1,"paid_at":AT}}),
        json!({"ticket_number":"INVALID-JOINT", "used_parts_cents":5000, "completed_labor_cents":8000, "payment":{"amount_cents":1,"paid_at":AT}, "pickup":null}),
        json!({"status":"Completed", "used_parts_cents":5000, "client_decision":null, "repair_checked":false}),
        json!({"used_parts_cents":5000, "completed_labor_cents":8000, "payment":{"amount_cents":11400,"paid_at":AT}, "pickup":null}),
    ] {
        reject(
            &app,
            &s,
            &order,
            body,
            StatusCode::CONFLICT,
            "GUARD_VIOLATION",
        )
        .await;
    }
    let fixed = patch(
        &app,
        &s.admin,
        &order,
        json!({"completed_labor_cents":8000,"payment":{"amount_cents":16400,"paid_at":AT}}),
    )
    .await
    .expect(StatusCode::OK);
    assert_eq!(fixed["status"], "Completed");
    assert_eq!(fixed["settlement_cents"], 16400);
    assert_eq!(fixed["labor_discount_cents"], 1600);
    assert_eq!(get_order(&app, &s.admin, &order).await, fixed);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn invalid_corrections_and_denials_change_neither_record_nor_audit(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    let other = received_order(&app, &s, "Standard").await;
    for token in [&s.recep, &s.tech] {
        let before = snapshot(&app, &s.admin, &order).await;
        patch(&app, token, &order, json!({"id":null}))
            .await
            .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
        assert_eq!(snapshot(&app, &s.admin, &order).await, before);
    }
    app.patch(&format!("/api/v1/orders/{}", order.id))
        .json(&json!({"ticket_number":"X"}))
        .send()
        .await
        .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
    for body in [
        json!({}),
        json!({"id":1}),
        json!({"version":1}),
        json!({"settlement_cents":0}),
        json!({"labor_discount_cents":0}),
        json!({"status":null}),
        json!({"ticket_number":null}),
        json!({"client_id":0}),
        json!({"ticket_number":" "}),
        json!({"completed_labor_cents":-1}),
        json!({"used_parts_cents":1.5}),
        json!({"repair_checked":null}),
        json!({"estimate":{"parts_cents":0}}),
        json!({"payment":{"amount_cents":0}}),
        json!({"pickup":{}}),
    ] {
        reject(
            &app,
            &s,
            &order,
            body,
            StatusCode::BAD_REQUEST,
            "VALIDATION_ERROR",
        )
        .await;
    }
    for body in [
        json!({"status":"InProgress"}),
        json!({"status":"Ready","repair_checked":true}),
        json!({"used_parts_cents":1}),
        json!({"repair_checked":true}),
        json!({"estimate":{"parts_cents":0,"labor_cents":0}}),
        json!({"payment":{"amount_cents":0,"paid_at":AT}}),
        json!({"pickup":{"picked_up_at":AT}}),
    ] {
        reject(
            &app,
            &s,
            &order,
            body,
            StatusCode::CONFLICT,
            "GUARD_VIOLATION",
        )
        .await;
    }
    reject(
        &app,
        &s,
        &order,
        json!({"ticket_number":other.ticket}),
        StatusCode::CONFLICT,
        "CONFLICT",
    )
    .await;
    reject(
        &app,
        &s,
        &order,
        json!({"client_id":other.client}),
        StatusCode::CONFLICT,
        "CONFLICT",
    )
    .await;
    reject(
        &app,
        &s,
        &order,
        json!({"assigned_technician_id":s.admin_id}),
        StatusCode::CONFLICT,
        "CONFLICT",
    )
    .await;
    reject(
        &app,
        &s,
        &order,
        json!({"device_id":i64::MAX}),
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
    )
    .await;
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn cancelled_corrections_preserve_source_and_stage_pricing(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    for stage in ["Received", "Diagnosing", "InProgress"] {
        let order = if stage == "InProgress" {
            in_progress_order(&app, &s, "Gold").await
        } else {
            received_order(&app, &s, "Gold").await
        };
        if stage == "Diagnosing" {
            diagnose(&app, &s, &order).await;
        }
        app.post(&order.path("cancel"))
            .token(&s.admin)
            .send()
            .await
            .expect(StatusCode::OK);
        let body = if stage == "InProgress" {
            json!({"completed_labor_cents":10000,"used_parts_cents":5000})
        } else if stage == "Diagnosing" {
            json!({"diagnostic_findings":null})
        } else {
            json!({"ticket_number":"CANCEL-CORRECTED"})
        };
        let result = patch(&app, &s.admin, &order, body)
            .await
            .expect(StatusCode::OK);
        assert_eq!(result["status"], "Cancelled");
        assert_eq!(
            result["settlement_cents"],
            if stage == "InProgress" { 13000 } else { 0 }
        );
        if stage == "Diagnosing" {
            let charged = patch(
                &app,
                &s.admin,
                &order,
                json!({"diagnostic_findings":"Corrected findings"}),
            )
            .await
            .expect(StatusCode::OK);
            assert_eq!(charged["settlement_cents"], 3000);
        }
        reject(
            &app,
            &s,
            &order,
            json!({"status":"Received"}),
            StatusCode::CONFLICT,
            "GUARD_VIOLATION",
        )
        .await;
        if stage != "InProgress" {
            reject(
                &app,
                &s,
                &order,
                json!({"completed_labor_cents":1}),
                StatusCode::CONFLICT,
                "GUARD_VIOLATION",
            )
            .await;
        }
    }
}
