mod common;

use axum::http::StatusCode;
use common::workshop::{
    self, Order, Staff, change, changes, get_order, in_progress_order, order_audit, put,
    received_order, repair, set_tier, snapshot, staff,
};
use common::{TestApp, TestResponse};
use serde_json::{Value, json};
use sqlx::PgPool;

const PAID: &str = "2026-10-23T15:00:00Z";
const PICKED: &str = "2026-10-23T15:05:00Z";

async fn command(app: &TestApp, token: &str, order: &Order, action: &str) -> TestResponse {
    app.post(&order.path(action)).token(token).send().await
}

async fn ready(app: &TestApp, s: &Staff, tier: &str, parts: i64, labor: i64) -> Order {
    let order = in_progress_order(app, s, tier).await;
    repair(app, s, &order, &json!({"status":"Ready", "used_parts_cents":parts, "completed_labor_cents":labor, "repair_checked":true})).await;
    order
}

async fn payment(app: &TestApp, token: &str, order: &Order, amount: i64) -> Value {
    put(
        app,
        token,
        order,
        "payment",
        &json!({"amount_cents":amount, "paid_at":PAID}),
    )
    .await
    .expect(StatusCode::OK)
}

async fn pickup(app: &TestApp, token: &str, order: &Order) -> Value {
    put(app, token, order, "pickup", &json!({"picked_up_at":PICKED}))
        .await
        .expect(StatusCode::OK)
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_005_006_007_013_018_cancellation_stages_and_roles(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    for (token, actor) in [(&s.recep, s.recep_id), (&s.admin, s.admin_id)] {
        for (stage, total, discount) in [
            ("Received", 0, 0),
            ("Unfinished", 0, 0),
            ("Findings", 3000, 0),
            ("Declined", 3000, 0),
            ("Approved", 3000, 0),
            ("ZeroRepair", 0, 0),
            ("Repair", 14000, 1000),
        ] {
            let order = received_order(&app, &s, "Gold").await;
            if stage != "Received" {
                put(&app, &s.tech, &order, "diagnosis", &json!({}))
                    .await
                    .expect(StatusCode::OK);
            }
            if !matches!(stage, "Received" | "Unfinished") {
                workshop::diagnose(&app, &s, &order).await;
            }
            if matches!(stage, "Declined" | "Approved" | "ZeroRepair" | "Repair") {
                workshop::estimate(&app, &s, &order, 10000, 10000).await;
                workshop::decide(
                    &app,
                    &s,
                    &order,
                    if stage == "Declined" {
                        "Declined"
                    } else {
                        "Approved"
                    },
                )
                .await;
            }
            if matches!(stage, "ZeroRepair" | "Repair") {
                repair(&app, &s, &order, &json!({"status":"InProgress"})).await;
            }
            if stage == "Repair" {
                repair(&app, &s, &order, &json!({"status":"InProgress", "used_parts_cents":10000, "completed_labor_cents":5000})).await;
            }
            let before = get_order(&app, &s.admin, &order).await;
            let audit_before = order_audit(&app, &s.admin, &order).await.len();
            let cancelled = command(&app, token, &order, "cancel")
                .await
                .expect(StatusCode::OK);
            assert_eq!(cancelled["status"], "Cancelled", "{stage}");
            assert_eq!(cancelled["settlement_cents"], total, "{stage}");
            assert_eq!(cancelled["labor_discount_cents"], discount, "{stage}");
            assert_eq!(workshop::keys(&cancelled), workshop::ORDER_KEYS);
            assert_eq!(get_order(&app, &s.recep, &order).await, cancelled);
            let audit = order_audit(&app, &s.admin, &order).await;
            assert_eq!(
                changes(&audit[audit_before..]),
                vec![
                    change(
                        "status",
                        before["status"].clone(),
                        json!("Cancelled"),
                        actor
                    ),
                    change(
                        "labor_discount_cents",
                        Value::Null,
                        json!(discount.to_string()),
                        actor
                    ),
                    change(
                        "settlement_cents",
                        Value::Null,
                        json!(total.to_string()),
                        actor
                    ),
                ]
            );
            set_tier(&app, &s, order.client, "Standard").await;
            assert_eq!(get_order(&app, &s.admin, &order).await, cancelled);
        }
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_008_014_015_020_completion_uses_current_tier_then_freezes(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    for (tier, parts, labor, discount, total) in [
        ("Standard", 10000, 10000, 0, 20000),
        ("Silver", 10000, 10000, 1000, 19000),
        ("Gold", 10000, 10000, 2000, 18000),
        ("Silver", 0, 5, 1, 4),
        ("Gold", 0, 0, 0, 0),
    ] {
        for (token, actor) in [(&s.recep, s.recep_id), (&s.admin, s.admin_id)] {
            let order = ready(&app, &s, "Standard", parts, labor).await;
            set_tier(&app, &s, order.client, tier).await;
            // Pickup does not require payment; neither command completes the order.
            assert_eq!(pickup(&app, token, &order).await["status"], "Ready");
            let paid = payment(&app, token, &order, total).await;
            assert_eq!(paid["status"], "Ready");
            assert_eq!(paid["labor_discount_cents"], discount);
            let audit_before = order_audit(&app, &s.admin, &order).await.len();
            let completed = command(&app, token, &order, "complete")
                .await
                .expect(StatusCode::OK);
            assert_eq!(completed["status"], "Completed");
            assert_eq!(completed["settlement_cents"], total);
            assert_eq!(
                completed["payment"],
                json!({"amount_cents":total, "paid_at":PAID})
            );
            assert_eq!(completed["pickup"], json!({"picked_up_at":PICKED}));
            let audit = order_audit(&app, &s.admin, &order).await;
            assert_eq!(
                changes(&audit[audit_before..]),
                vec![
                    change("status", json!("Ready"), json!("Completed"), actor),
                    change(
                        "labor_discount_cents",
                        Value::Null,
                        json!(discount.to_string()),
                        actor
                    ),
                    change(
                        "settlement_cents",
                        Value::Null,
                        json!(total.to_string()),
                        actor
                    ),
                ]
            );
            set_tier(&app, &s, order.client, "Gold").await;
            assert_eq!(get_order(&app, &s.admin, &order).await, completed);
            let list = app
                .get(&format!("/api/v1/orders?client_id={}", order.client))
                .token(&s.recep)
                .send()
                .await
                .expect(StatusCode::OK);
            assert_eq!(list[0], completed);
        }
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_010_012_exact_payment_and_pickup_guards_apply_to_admin(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    for token in [&s.recep, &s.admin] {
        let order = ready(&app, &s, "Silver", 10000, 10000).await;
        for amount in [None, Some(10000), Some(19001), Some(19000)] {
            if let Some(amount) = amount {
                payment(&app, token, &order, amount).await;
            }
            let before = snapshot(&app, &s.admin, &order).await;
            command(&app, token, &order, "complete")
                .await
                .expect_error(StatusCode::CONFLICT, "GUARD_VIOLATION");
            assert_eq!(snapshot(&app, &s.admin, &order).await, before);
        }
        pickup(&app, token, &order).await;
        for amount in [10000, 19001] {
            payment(&app, token, &order, amount).await;
            let before = snapshot(&app, &s.admin, &order).await;
            let err = command(&app, token, &order, "complete")
                .await
                .expect_error(StatusCode::CONFLICT, "GUARD_VIOLATION");
            assert_eq!(err["details"]["paid_cents"], amount);
            assert_eq!(err["details"]["settlement_cents"], 19000);
            assert_eq!(snapshot(&app, &s.admin, &order).await, before);
        }
        payment(&app, token, &order, 19000).await;
        // Tier changes invalidate a previously exact payment, including overpayment.
        set_tier(&app, &s, order.client, "Gold").await;
        let before = snapshot(&app, &s.admin, &order).await;
        let err = command(&app, token, &order, "complete")
            .await
            .expect_error(StatusCode::CONFLICT, "GUARD_VIOLATION");
        assert_eq!(err["details"]["paid_cents"], 19000);
        assert_eq!(err["details"]["settlement_cents"], 18000);
        assert_eq!(snapshot(&app, &s.admin, &order).await, before);
        payment(&app, token, &order, 18000).await;
        command(&app, token, &order, "complete")
            .await
            .expect(StatusCode::OK);
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn payment_pickup_validate_shapes_and_normalize_audited_timestamps(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = ready(&app, &s, "Silver", 10000, 10000).await;
    for (endpoint, bodies) in [
        (
            "payment",
            vec![
                json!({}),
                json!({"amount_cents":0}),
                json!({"paid_at":PAID}),
                json!({"amount_cents":null,"paid_at":PAID}),
                json!({"amount_cents":0,"paid_at":null}),
                json!({"amount_cents":-1,"paid_at":PAID}),
                json!({"amount_cents":0.5,"paid_at":PAID}),
                json!({"amount_cents":0,"paid_at":"bad"}),
                json!({"amount_cents":0,"paid_at":PAID,"extra":true}),
            ],
        ),
        (
            "pickup",
            vec![
                json!({}),
                json!({"picked_up_at":null}),
                json!({"picked_up_at":"bad"}),
                json!({"picked_up_at":PICKED,"extra":true}),
            ],
        ),
    ] {
        for body in bodies {
            let before = snapshot(&app, &s.admin, &order).await;
            put(&app, &s.recep, &order, endpoint, &body)
                .await
                .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
            assert_eq!(snapshot(&app, &s.admin, &order).await, before);
        }
    }
    for (endpoint, body, normalized, field) in [
        (
            "payment",
            json!({"amount_cents":19000,"paid_at":"2026-10-23T17:00:00.123456789+02:00"}),
            json!({"amount_cents":19000,"paid_at":"2026-10-23T15:00:00.123456Z"}),
            "payment.paid_at",
        ),
        (
            "pickup",
            json!({"picked_up_at":"2026-10-23T17:05:00.987654321+02:00"}),
            json!({"picked_up_at":"2026-10-23T15:05:00.987654Z"}),
            "pickup.picked_up_at",
        ),
    ] {
        let recorded = put(&app, &s.recep, &order, endpoint, &body)
            .await
            .expect(StatusCode::OK);
        assert_eq!(recorded[endpoint], normalized);
        assert_eq!(get_order(&app, &s.admin, &order).await, recorded);
        let audit = order_audit(&app, &s.admin, &order).await;
        let entry = audit.iter().rev().find(|e| e["field"] == field).unwrap();
        assert_eq!(
            entry["new_value"],
            normalized[if endpoint == "payment" {
                "paid_at"
            } else {
                "picked_up_at"
            }]
        );
        assert_eq!(entry["actor_id"], s.recep_id);
        let before = snapshot(&app, &s.admin, &order).await;
        put(&app, &s.recep, &order, endpoint, &body)
            .await
            .expect(StatusCode::OK);
        assert_eq!(snapshot(&app, &s.admin, &order).await, before);
    }
    let audit = order_audit(&app, &s.admin, &order).await;
    assert!(
        audit
            .iter()
            .any(|e| e["field"] == "payment.amount_cents" && e["new_value"] == "19000")
    );
    payment(&app, &s.admin, &order, 18000).await;
    let audit = order_audit(&app, &s.admin, &order).await;
    let amount = audit
        .iter()
        .rev()
        .find(|e| e["field"] == "payment.amount_cents")
        .unwrap();
    assert_eq!(amount["previous_value"], "19000");
    assert_eq!(amount["new_value"], "18000");
    assert_eq!(amount["actor_id"], s.admin_id);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_019_terminal_and_wrong_stage_commands_are_unchanged(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    for stage in [
        "Received",
        "Diagnosing",
        "InProgress",
        "Ready",
        "Completed",
        "Cancelled",
    ] {
        let order = received_order(&app, &s, "Standard").await;
        if matches!(stage, "Diagnosing" | "InProgress" | "Ready" | "Completed") {
            workshop::diagnose(&app, &s, &order).await;
        }
        if matches!(stage, "InProgress" | "Ready" | "Completed") {
            workshop::estimate(&app, &s, &order, 0, 0).await;
            workshop::decide(&app, &s, &order, "Approved").await;
            repair(&app, &s, &order, &json!({"status":"InProgress"})).await;
        }
        if matches!(stage, "Ready" | "Completed") {
            repair(
                &app,
                &s,
                &order,
                &json!({"status":"Ready","repair_checked":true}),
            )
            .await;
        }
        if stage == "Completed" {
            payment(&app, &s.recep, &order, 0).await;
            pickup(&app, &s.recep, &order).await;
            command(&app, &s.recep, &order, "complete")
                .await
                .expect(StatusCode::OK);
        }
        if stage == "Cancelled" {
            command(&app, &s.recep, &order, "cancel")
                .await
                .expect(StatusCode::OK);
        }
        if matches!(stage, "Completed" | "Cancelled") {
            let before = snapshot(&app, &s.admin, &order).await;
            for (endpoint, body) in [
                ("diagnosis", json!({"diagnostic_findings":"Changed"})),
                ("estimate", json!({"parts_cents":1,"labor_cents":2})),
                ("decision", json!({"decision":"Approved","decided_at":PAID})),
                (
                    "repair",
                    json!({"status":"InProgress","used_parts_cents":1}),
                ),
            ] {
                put(&app, &s.admin, &order, endpoint, &body)
                    .await
                    .expect_error(StatusCode::CONFLICT, "GUARD_VIOLATION");
                assert_eq!(snapshot(&app, &s.admin, &order).await, before);
            }
        }
        let before = snapshot(&app, &s.admin, &order).await;
        for token in [&s.recep, &s.admin, &s.tech] {
            for (endpoint, body) in [
                ("payment", json!({"amount_cents":0,"paid_at":PAID})),
                ("pickup", json!({"picked_up_at":PICKED})),
            ] {
                if token != &s.tech && stage == "Ready" {
                    continue;
                }
                put(&app, token, &order, endpoint, &body)
                    .await
                    .expect_error(
                        if token == &s.tech {
                            StatusCode::FORBIDDEN
                        } else {
                            StatusCode::CONFLICT
                        },
                        if token == &s.tech {
                            "FORBIDDEN"
                        } else {
                            "GUARD_VIOLATION"
                        },
                    );
                assert_eq!(snapshot(&app, &s.admin, &order).await, before);
            }
            for endpoint in ["complete", "cancel"] {
                if token != &s.tech
                    && endpoint == "cancel"
                    && matches!(stage, "Received" | "Diagnosing" | "InProgress")
                {
                    continue;
                }
                command(&app, token, &order, endpoint).await.expect_error(
                    if token == &s.tech {
                        StatusCode::FORBIDDEN
                    } else {
                        StatusCode::CONFLICT
                    },
                    if token == &s.tech {
                        "FORBIDDEN"
                    } else {
                        "GUARD_VIOLATION"
                    },
                );
                assert_eq!(snapshot(&app, &s.admin, &order).await, before);
            }
        }
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn settlement_auth_validation_and_missing_order_precedence(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    for endpoint in ["payment", "pickup", "complete", "cancel"] {
        let path = format!("/api/v1/orders/not-an-id/{endpoint}");
        let method = if matches!(endpoint, "payment" | "pickup") {
            axum::http::Method::PUT
        } else {
            axum::http::Method::POST
        };
        app.request(method.clone(), &path)
            .raw_json("{")
            .send()
            .await
            .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
        app.request(method.clone(), &path)
            .token(&s.tech)
            .raw_json("{")
            .send()
            .await
            .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
        app.request(method.clone(), &path)
            .token(&s.admin)
            .raw_json("{")
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        let path = format!("/api/v1/orders/99999/{endpoint}");
        let mut request = app.request(method, &path).token(&s.recep);
        if endpoint == "payment" {
            request = request.json(&json!({"amount_cents":0,"paid_at":PAID}));
        }
        if endpoint == "pickup" {
            request = request.json(&json!({"picked_up_at":PICKED}));
        }
        request
            .send()
            .await
            .expect_error(StatusCode::NOT_FOUND, "NOT_FOUND");
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn settlement_rejects_stale_order_and_client_and_rolls_back_failed_audit(pool: PgPool) {
    use fixops_app::{StoreError, clients::ClientStore, orders::OrderStore};
    use fixops_domain::{
        id::{ClientId, OrderId, StaffId},
        money::Cents,
        order::Payment,
    };
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = ready(&app, &s, "Silver", 10000, 10000).await;
    payment(&app, &s.recep, &order, 19000).await;
    pickup(&app, &s.recep, &order).await;
    let store = app.app().store();
    let stale = store.find_order(OrderId(order.id)).await.unwrap().unwrap();
    let (client, version) = store
        .find_client_with_version(ClientId(order.client))
        .await
        .unwrap()
        .unwrap();
    let (completed, audit) = stale.complete(client.loyalty_tier).unwrap();
    set_tier(&app, &s, order.client, "Gold").await;
    let before = snapshot(&app, &s.admin, &order).await;
    let err = store
        .settle_order(StaffId(s.recep_id), &completed, version, &audit)
        .await
        .unwrap_err();
    assert!(matches!(err, StoreError::Stale));
    assert_eq!(snapshot(&app, &s.admin, &order).await, before);
    let (changed, audit) = stale
        .record_payment(Payment {
            amount: Cents::new("amount_cents", 18000).unwrap(),
            paid_at: OffsetDateTime::parse(PAID, &Rfc3339).unwrap(),
        })
        .unwrap();
    store
        .update_order(StaffId(s.recep_id), &changed, &audit)
        .await
        .unwrap();
    let (_, version) = store
        .find_client_with_version(ClientId(order.client))
        .await
        .unwrap()
        .unwrap();
    let before = snapshot(&app, &s.admin, &order).await;
    assert!(matches!(
        store
            .settle_order(StaffId(s.recep_id), &completed, version, &[])
            .await
            .unwrap_err(),
        StoreError::Stale
    ));
    assert_eq!(snapshot(&app, &s.admin, &order).await, before);
    assert!(matches!(
        store
            .update_order(StaffId(s.recep_id), &changed, &audit)
            .await
            .unwrap_err(),
        StoreError::Stale
    ));
    assert_eq!(snapshot(&app, &s.admin, &order).await, before);
    let current = store.find_order(OrderId(order.id)).await.unwrap().unwrap();
    let (completed, audit) = current
        .complete(fixops_domain::client::LoyaltyTier::Gold)
        .unwrap();
    // An invalid actor forces audit insertion to fail after the versioned order UPDATE.
    assert!(
        store
            .settle_order(StaffId(i64::MAX), &completed, version, &audit)
            .await
            .is_err()
    );
    assert_eq!(snapshot(&app, &s.admin, &order).await, before);
    store
        .settle_order(StaffId(s.admin_id), &completed, version, &audit)
        .await
        .unwrap();
    let stored = store.find_order(OrderId(order.id)).await.unwrap().unwrap();
    assert_eq!(stored.final_settlement.unwrap().total.get(), 18000);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn cancellation_retains_lossless_source_stage(pool: PgPool) {
    use fixops_app::orders::OrderStore;
    use fixops_domain::{id::OrderId, order::OrderStatus};
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    for stage in [
        OrderStatus::Received,
        OrderStatus::Diagnosing,
        OrderStatus::InProgress,
    ] {
        let order = match stage {
            OrderStatus::Received => received_order(&app, &s, "Standard").await,
            OrderStatus::Diagnosing => {
                workshop::approved_order(&app, &s, "Standard", 10000, 10000).await
            }
            _ => in_progress_order(&app, &s, "Standard").await,
        };
        command(&app, &s.recep, &order, "cancel")
            .await
            .expect(StatusCode::OK);
        let saved = app
            .app()
            .store()
            .find_order(OrderId(order.id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.cancelled_from, Some(stage));
        assert_eq!(
            saved.final_settlement.unwrap().total.get(),
            if stage == OrderStatus::Diagnosing {
                3000
            } else {
                0
            }
        );
    }
}
