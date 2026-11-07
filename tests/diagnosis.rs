//! Diagnosis, estimate, client decision and the approval gate on repair
//! (REQ-F-006–009, REQ-F-002, REQ-NF-002, REQ-NF-003; BDD-003, BDD-004, BDD-006, BDD-013, BDD-020).

mod common;

use axum::http::{Method, StatusCode};
use common::TestApp;
use common::workshop::{
    DECIDED_AT, FINDINGS, Order, Staff, approved_order, change, changes, decide, diagnose,
    estimate, get_order, keys, order_audit, put, received_order, snapshot, staff,
};
use serde_json::{Value, json};
use sqlx::PgPool;

fn approval() -> Value {
    json!({ "decision": "Approved", "decided_at": DECIDED_AT })
}

fn approved_fields(status: &str) -> Value {
    json!({
        "status": status,
        "diagnostic_findings": FINDINGS,
        "estimate": { "parts_cents": 10000, "labor_cents": 10000 },
        "client_decision": { "decision": "Approved", "decided_at": DECIDED_AT }
    })
}

// --- BDD-003: approval gates repair (REQ-F-007–009) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_003_approval_is_recorded(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;

    let diagnosing = diagnose(&app, &s, &order).await;
    assert_eq!(
        diagnosing,
        order.expected(&json!({ "status": "Diagnosing", "diagnostic_findings": FINDINGS }))
    );

    let estimated = estimate(&app, &s, &order, 10_000, 10_000).await;
    assert_eq!(
        estimated,
        order.expected(&json!({
            "status": "Diagnosing",
            "diagnostic_findings": FINDINGS,
            "estimate": { "parts_cents": 10000, "labor_cents": 10000 }
        }))
    );

    let approved = decide(&app, &s, &order, "Approved").await;
    assert_eq!(approved, order.expected(&approved_fields("Diagnosing")));

    assert_eq!(get_order(&app, &s.recep, &order).await, approved);
}

/// BDD-020: an Administrator performs every Technician and Receptionist step, and the audit
/// trail names the Administrator.
#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_020_administrator_runs_diagnosis_estimate_and_decision(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    let intake_entries = order_audit(&app, &s.admin, &order).await.len();

    for (endpoint, body) in [
        ("diagnosis", json!({ "diagnostic_findings": FINDINGS })),
        (
            "estimate",
            json!({ "parts_cents": 10000, "labor_cents": 10000 }),
        ),
        ("decision", approval()),
    ] {
        put(&app, &s.admin, &order, endpoint, &body)
            .await
            .expect(StatusCode::OK);
    }
    assert_eq!(
        get_order(&app, &s.admin, &order).await,
        order.expected(&approved_fields("Diagnosing"))
    );
    let entries = order_audit(&app, &s.admin, &order).await;
    let new = &entries[intake_entries..];
    assert_eq!(new.len(), 6, "{new:#?}");
    assert!(new.iter().all(|e| e["actor_id"] == s.admin_id), "{new:#?}");
}

// --- BDD-006 (decision part), REQ-F-008 ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_006_declined_estimate_is_recorded_with_its_time(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    diagnose(&app, &s, &order).await;
    estimate(&app, &s, &order, 10_000, 10_000).await;

    let declined = decide(&app, &s, &order, "Declined").await;
    assert_eq!(
        declined,
        order.expected(&json!({
            "status": "Diagnosing",
            "diagnostic_findings": FINDINGS,
            "estimate": { "parts_cents": 10000, "labor_cents": 10000 },
            "client_decision": { "decision": "Declined", "decided_at": DECIDED_AT }
        }))
    );
    assert_eq!(get_order(&app, &s.tech, &order).await, declined);
}

// --- Diagnosis (REQ-F-006, BDD-013) ---

/// BDD-013 precondition: diagnosis starts without findings; they are saved later.
#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_013_diagnosis_starts_without_findings_and_saves_them_later(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;

    let started = put(&app, &s.tech, &order, "diagnosis", &json!({}))
        .await
        .expect(StatusCode::OK);
    assert_eq!(started, order.expected(&json!({ "status": "Diagnosing" })));

    let saved = diagnose(&app, &s, &order).await;
    assert_eq!(
        saved,
        order.expected(&json!({ "status": "Diagnosing", "diagnostic_findings": FINDINGS }))
    );

    let revised = put(
        &app,
        &s.tech,
        &order,
        "diagnosis",
        &json!({ "diagnostic_findings": "  Loose fan connector  " }),
    )
    .await
    .expect(StatusCode::OK);
    assert_eq!(revised["diagnostic_findings"], "Loose fan connector");
    assert_eq!(revised["status"], "Diagnosing");
}

/// `diagnostic_findings` is nullable: `null` and absence both save no findings and keep what
/// is saved.
#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn null_and_absent_findings_save_nothing(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;

    let order = received_order(&app, &s, "Standard").await;
    let started = put(
        &app,
        &s.tech,
        &order,
        "diagnosis",
        &json!({ "diagnostic_findings": null }),
    )
    .await
    .expect(StatusCode::OK);
    assert_eq!(started, order.expected(&json!({ "status": "Diagnosing" })));

    let saved = received_order(&app, &s, "Standard").await;
    diagnose(&app, &s, &saved).await;
    let before = snapshot(&app, &s.admin, &saved).await;
    for body in [json!({}), json!({ "diagnostic_findings": null })] {
        let kept = put(&app, &s.tech, &saved, "diagnosis", &body)
            .await
            .expect(StatusCode::OK);
        assert_eq!(kept["diagnostic_findings"], FINDINGS);
    }
    assert_eq!(snapshot(&app, &s.admin, &saved).await, before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn invalid_findings_are_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    let before = snapshot(&app, &s.admin, &order).await;

    for blank in ["", "   "] {
        let body = put(
            &app,
            &s.tech,
            &order,
            "diagnosis",
            &json!({ "diagnostic_findings": blank }),
        )
        .await
        .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        assert_eq!(body["details"]["field"], "diagnostic_findings");
    }
    for body in [
        json!({ "diagnostic_findings": 5 }),
        json!({ "diagnostic_findings": ["x"] }),
        json!({ "diagnostic_findings": FINDINGS, "status": "Diagnosing" }),
        json!([]),
        json!("findings"),
    ] {
        put(&app, &s.tech, &order, "diagnosis", &body)
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
    app.put(&order.path("diagnosis"))
        .token(&s.tech)
        .raw_json("{not json")
        .send()
        .await
        .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    assert_eq!(snapshot(&app, &s.admin, &order).await, before);
}

/// Findings are the basis of the decided estimate, so they are fixed once a decision is
/// recorded; later states cannot go back to `Diagnosing` (SRS §3.1).
#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn diagnosis_is_rejected_after_a_decision(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let decided = approved_order(&app, &s, "Standard", 10_000, 10_000).await;
    for order in [&decided] {
        let before = snapshot(&app, &s.admin, order).await;
        for token in [&s.tech, &s.admin] {
            for body in [json!({ "diagnostic_findings": "Other" }), json!({})] {
                put(&app, token, order, "diagnosis", &body)
                    .await
                    .expect_error(StatusCode::CONFLICT, "GUARD_VIOLATION");
            }
        }
        assert_eq!(snapshot(&app, &s.admin, order).await, before);
    }
}

// --- Estimate (REQ-F-007) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn estimate_requires_saved_findings(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let received = received_order(&app, &s, "Standard").await;
    let no_findings = received_order(&app, &s, "Standard").await;
    put(&app, &s.tech, &no_findings, "diagnosis", &json!({}))
        .await
        .expect(StatusCode::OK);

    for order in [&received, &no_findings] {
        let before = snapshot(&app, &s.admin, order).await;
        for token in [&s.tech, &s.admin] {
            let body = put(
                &app,
                token,
                order,
                "estimate",
                &json!({ "parts_cents": 10000, "labor_cents": 10000 }),
            )
            .await
            .expect_error(StatusCode::CONFLICT, "GUARD_VIOLATION");
            assert_eq!(
                body["message"],
                "Diagnostic findings must be saved before preparing an estimate"
            );
        }
        assert_eq!(snapshot(&app, &s.admin, order).await, before);
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn estimate_is_rejected_after_a_client_decision(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let approved = approved_order(&app, &s, "Standard", 10_000, 10_000).await;
    let declined = received_order(&app, &s, "Standard").await;
    diagnose(&app, &s, &declined).await;
    estimate(&app, &s, &declined, 10_000, 10_000).await;
    decide(&app, &s, &declined, "Declined").await;

    for order in [&approved, &declined] {
        let before = snapshot(&app, &s.admin, order).await;
        for token in [&s.tech, &s.admin] {
            put(
                &app,
                token,
                order,
                "estimate",
                &json!({ "parts_cents": 5000, "labor_cents": 5000 }),
            )
            .await
            .expect_error(StatusCode::CONFLICT, "GUARD_VIOLATION");
        }
        assert_eq!(snapshot(&app, &s.admin, order).await, before);
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn estimate_can_be_revised_before_a_decision(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    diagnose(&app, &s, &order).await;
    estimate(&app, &s, &order, 10_000, 10_000).await;
    let entries = order_audit(&app, &s.admin, &order).await.len();

    let revised = estimate(&app, &s, &order, 10_000, 8_000).await;
    assert_eq!(
        revised["estimate"],
        json!({ "parts_cents": 10000, "labor_cents": 8000 })
    );
    let after = order_audit(&app, &s.admin, &order).await;
    assert_eq!(
        changes(&after[entries..]),
        [change(
            "estimate.labor_cents",
            json!("10000"),
            json!("8000"),
            s.tech_id
        )]
    );
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn invalid_estimates_are_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    diagnose(&app, &s, &order).await;
    let before = snapshot(&app, &s.admin, &order).await;

    for (field, body) in [
        (
            "parts_cents",
            json!({ "parts_cents": -1, "labor_cents": 100 }),
        ),
        (
            "labor_cents",
            json!({ "parts_cents": 100, "labor_cents": -1 }),
        ),
    ] {
        let error = put(&app, &s.tech, &order, "estimate", &body)
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        assert_eq!(error["details"]["field"], field);
    }
    for body in [
        json!({}),
        json!({ "parts_cents": 100 }),
        json!({ "labor_cents": 100 }),
        json!({ "parts_cents": null, "labor_cents": 100 }),
        json!({ "parts_cents": 100, "labor_cents": null }),
        json!({ "parts_cents": 1.5, "labor_cents": 100 }),
        json!({ "parts_cents": "100", "labor_cents": 100 }),
        json!({ "parts_cents": 100, "labor_cents": 100, "total_cents": 200 }),
        json!(null),
    ] {
        put(&app, &s.tech, &order, "estimate", &body)
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
    assert_eq!(snapshot(&app, &s.admin, &order).await, before);
}

// --- Client decision (REQ-F-008) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn decision_without_a_valid_time_or_answer_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    diagnose(&app, &s, &order).await;
    estimate(&app, &s, &order, 10_000, 10_000).await;
    let before = snapshot(&app, &s.admin, &order).await;

    for body in [
        json!({ "decision": "Approved" }),
        json!({ "decision": "Approved", "decided_at": null }),
        json!({ "decision": "Approved", "decided_at": "" }),
        json!({ "decision": "Approved", "decided_at": "yesterday" }),
        json!({ "decision": "Approved", "decided_at": "2026-10-21" }),
        json!({ "decision": "Approved", "decided_at": 1_792_577_700 }),
        json!({ "decided_at": DECIDED_AT }),
        json!({ "decision": null, "decided_at": DECIDED_AT }),
        json!({ "decision": "approved", "decided_at": DECIDED_AT }),
        json!({ "decision": "Maybe", "decided_at": DECIDED_AT }),
        json!({ "decision": "Approved", "decided_at": DECIDED_AT, "note": "ok" }),
        json!({}),
    ] {
        for token in [&s.recep, &s.admin] {
            put(&app, token, &order, "decision", &body)
                .await
                .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        }
    }
    assert_eq!(snapshot(&app, &s.admin, &order).await, before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn decision_requires_an_estimate(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let received = received_order(&app, &s, "Standard").await;
    let no_findings = received_order(&app, &s, "Standard").await;
    put(&app, &s.tech, &no_findings, "diagnosis", &json!({}))
        .await
        .expect(StatusCode::OK);
    let no_estimate = received_order(&app, &s, "Standard").await;
    diagnose(&app, &s, &no_estimate).await;

    for order in [&received, &no_findings, &no_estimate] {
        let before = snapshot(&app, &s.admin, order).await;
        for token in [&s.recep, &s.admin] {
            put(&app, token, order, "decision", &approval())
                .await
                .expect_error(StatusCode::CONFLICT, "GUARD_VIOLATION");
        }
        assert_eq!(snapshot(&app, &s.admin, order).await, before);
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn decision_is_recorded_once(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    diagnose(&app, &s, &order).await;
    estimate(&app, &s, &order, 10_000, 10_000).await;
    decide(&app, &s, &order, "Declined").await;
    let before = snapshot(&app, &s.admin, &order).await;

    for token in [&s.recep, &s.admin] {
        put(&app, token, &order, "decision", &approval())
            .await
            .expect_error(StatusCode::CONFLICT, "GUARD_VIOLATION");
    }
    assert_eq!(snapshot(&app, &s.admin, &order).await, before);
}

// --- Audit (REQ-NF-003) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn each_step_is_audited_with_old_and_new_values(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    let intake_entries = order_audit(&app, &s.admin, &order).await.len();

    approved_order_steps(&app, &s, &order).await;

    let entries = order_audit(&app, &s.recep, &order).await;
    assert_eq!(
        changes(&entries[intake_entries..]),
        [
            change("status", json!("Received"), json!("Diagnosing"), s.tech_id),
            change(
                "diagnostic_findings",
                Value::Null,
                json!(FINDINGS),
                s.tech_id
            ),
            change(
                "estimate.parts_cents",
                Value::Null,
                json!("10000"),
                s.tech_id
            ),
            change(
                "estimate.labor_cents",
                Value::Null,
                json!("10000"),
                s.tech_id
            ),
            change(
                "client_decision.decision",
                Value::Null,
                json!("Approved"),
                s.recep_id
            ),
            change(
                "client_decision.decided_at",
                Value::Null,
                json!(DECIDED_AT),
                s.recep_id
            ),
        ]
    );
    for entry in &entries[intake_entries..] {
        assert_eq!(entry["record_type"], "RepairOrder");
        assert_eq!(entry["record_id"], order.id);
        assert!(entry["recorded_at"].as_str().is_some_and(|t| !t.is_empty()));
    }
}

async fn approved_order_steps(app: &TestApp, s: &Staff, order: &Order) {
    diagnose(app, s, order).await;
    estimate(app, s, order, 10_000, 10_000).await;
    decide(app, s, order, "Approved").await;
}

// --- Roles (REQ-F-002, REQ-NF-002, ADD-001) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn technician_cannot_record_a_client_decision(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    diagnose(&app, &s, &order).await;
    estimate(&app, &s, &order, 10_000, 10_000).await;
    let before = snapshot(&app, &s.admin, &order).await;

    for (path, body) in [
        (order.path("decision"), approval()),
        (order.path("decision"), json!({ "decision": "Maybe" })),
        ("/api/v1/orders/0/decision".to_owned(), approval()),
        ("/api/v1/orders/abc/decision".to_owned(), json!({})),
        ("/api/v1/orders/999999/decision".to_owned(), approval()),
    ] {
        app.put(&path)
            .token(&s.tech)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
    }
    app.put(&order.path("decision"))
        .token(&s.tech)
        .raw_json("{not json")
        .send()
        .await
        .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
    assert_eq!(snapshot(&app, &s.admin, &order).await, before);
}

/// The role is checked before the path and body (401 → 403 → 400 → 404/409).
#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn receptionist_cannot_diagnose_or_estimate(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let received = received_order(&app, &s, "Standard").await;
    let approved = approved_order(&app, &s, "Standard", 10_000, 10_000).await;
    let before = (
        snapshot(&app, &s.admin, &received).await,
        snapshot(&app, &s.admin, &approved).await,
    );

    let requests = [
        (
            received.path("diagnosis"),
            json!({ "diagnostic_findings": FINDINGS }),
        ),
        (
            received.path("diagnosis"),
            json!({ "diagnostic_findings": "" }),
        ),
        ("/api/v1/orders/0/diagnosis".to_owned(), json!({})),
        ("/api/v1/orders/999999/diagnosis".to_owned(), json!({})),
        (
            approved.path("estimate"),
            json!({ "parts_cents": 1, "labor_cents": 1 }),
        ),
        (approved.path("estimate"), json!({ "parts_cents": -1 })),
        ("/api/v1/orders/abc/estimate".to_owned(), json!({})),
    ];
    for (path, body) in requests {
        app.put(&path)
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
    }
    assert_eq!(
        (
            snapshot(&app, &s.admin, &received).await,
            snapshot(&app, &s.admin, &approved).await,
        ),
        before
    );
}

/// REQ-F-001: every new operation needs a staff session.
#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn sign_in_is_required_for_diagnosis_estimate_and_decision(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    let before = snapshot(&app, &s.admin, &order).await;

    for (endpoint, body) in [
        ("diagnosis", json!({ "diagnostic_findings": FINDINGS })),
        ("estimate", json!({ "parts_cents": 1, "labor_cents": 1 })),
        ("decision", approval()),
    ] {
        let no_token = app
            .request(Method::PUT, &order.path(endpoint))
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
        assert_eq!(keys(&no_token), ["code", "message"]);
        app.put(&order.path(endpoint))
            .token("not-a-session")
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
    }
    assert_eq!(snapshot(&app, &s.admin, &order).await, before);
}

/// ADD-002 §5: an unknown order is `404`; an invalid id or body is `400` first.
#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn unknown_orders_are_not_found(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let order = received_order(&app, &s, "Standard").await;
    // The endpoints exist: a known order is diagnosed.
    diagnose(&app, &s, &order).await;

    let requests = [
        (
            "diagnosis",
            json!({ "diagnostic_findings": FINDINGS }),
            &s.tech,
        ),
        (
            "estimate",
            json!({ "parts_cents": 1, "labor_cents": 1 }),
            &s.tech,
        ),
        ("decision", approval(), &s.recep),
    ];
    for (endpoint, body, token) in &requests {
        for token in [*token, &s.admin] {
            app.put(&format!("/api/v1/orders/999999/{endpoint}"))
                .token(token)
                .json(body)
                .send()
                .await
                .expect_error(StatusCode::NOT_FOUND, "NOT_FOUND");
            for bad in ["0", "-1", "abc", "1.5"] {
                let error = app
                    .put(&format!("/api/v1/orders/{bad}/{endpoint}"))
                    .token(token)
                    .json(body)
                    .send()
                    .await
                    .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
                assert_eq!(error["details"]["field"], "orderId");
            }
            app.put(&format!("/api/v1/orders/999999/{endpoint}"))
                .token(token)
                .json(&json!({ "unknown": true }))
                .send()
                .await
                .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        }
    }
}
