//! API support for the browser interface (ADD-004): cookie sessions, the staff list for every
//! role, order work queues and paging (REQ-F-001, REQ-F-002, REQ-F-022, REQ-F-023, REQ-NF-002;
//! BDD-022, BDD-023, BDD-024).

mod common;

use axum::http::{StatusCode, header};
use common::workshop::{
    Order, diagnose, estimate, in_progress_order, received_order, repair, staff,
};
use common::{PASSWORD, TestApp, TestResponse};
use serde_json::{Value, json};
use sqlx::PgPool;

fn ids(body: &Value) -> Vec<i64> {
    body.as_array()
        .expect("JSON array")
        .iter()
        .map(|item| item["id"].as_i64().expect("id"))
        .collect()
}

fn total(response: &TestResponse) -> i64 {
    response.headers["x-total-count"]
        .to_str()
        .expect("ASCII header")
        .parse()
        .expect("integer count")
}

// --- BDD-022: browser session (REQ-F-001, REQ-F-022) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_022_sign_in_sets_an_http_only_cookie_that_sign_out_ends(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    app.create_staff(&admin, "anna.recep", "Receptionist").await;

    let response = app
        .post("/api/v1/sessions")
        .json(&json!({ "username": "anna.recep", "password": PASSWORD }))
        .send()
        .await;
    assert_eq!(response.status, StatusCode::CREATED);
    let token = response.body["token"].as_str().unwrap().to_owned();
    let cookie = response.headers[header::SET_COOKIE].to_str().unwrap();
    assert!(
        cookie.starts_with(&format!("fixops_session={token};")),
        "{cookie}"
    );
    for attribute in [
        "HttpOnly",
        "Secure",
        "SameSite=Strict",
        "Path=/",
        "Max-Age=43200",
    ] {
        assert!(cookie.contains(attribute), "{cookie} lacks {attribute}");
    }

    let me = app
        .get("/api/v1/sessions/current")
        .cookie(&token)
        .send()
        .await
        .expect(StatusCode::OK);
    assert_eq!(me["username"], "anna.recep");
    assert_eq!(me["role"], "Receptionist");
    app.get("/api/v1/clients")
        .cookie(&token)
        .send()
        .await
        .expect(StatusCode::OK);
    app.get("/api/v1/sessions/current")
        .token(&token)
        .send()
        .await
        .expect(StatusCode::OK);

    let response = app
        .request(axum::http::Method::DELETE, "/api/v1/sessions/current")
        .cookie(&token)
        .send()
        .await;
    assert_eq!(response.status, StatusCode::NO_CONTENT);
    let cleared = response.headers[header::SET_COOKIE].to_str().unwrap();
    assert!(cleared.starts_with("fixops_session=;"), "{cleared}");
    assert!(cleared.contains("Max-Age=0"), "{cleared}");

    for request in [
        app.get("/api/v1/sessions/current").cookie(&token),
        app.get("/api/v1/clients").cookie(&token),
        app.get("/api/v1/clients").token(&token),
    ] {
        request
            .send()
            .await
            .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn current_session_and_sign_out_need_a_session(pool: PgPool) {
    let app = TestApp::new(pool);
    app.get("/api/v1/sessions/current")
        .send()
        .await
        .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
    app.request(axum::http::Method::DELETE, "/api/v1/sessions/current")
        .cookie("not-a-session")
        .send()
        .await
        .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
}

// --- BDD-024: choose a Technician at intake (REQ-F-002, REQ-NF-002) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_024_every_role_lists_staff_but_only_admin_manages(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let away = app.create_staff(&s.admin, "away.tech", "Technician").await;
    app.patch(&format!("/api/v1/staff/{away}"))
        .token(&s.admin)
        .json(&json!({ "enabled": false }))
        .send()
        .await
        .expect(StatusCode::OK);

    let technicians = app
        .get("/api/v1/staff?role=Technician&enabled=true")
        .token(&s.recep)
        .send()
        .await
        .expect(StatusCode::OK);
    assert_eq!(
        technicians,
        json!([{ "id": s.tech_id, "username": "max.tech", "role": "Technician", "enabled": true }])
    );
    let everyone = app.get("/api/v1/staff").token(&s.tech).send().await;
    assert_eq!(everyone.status, StatusCode::OK);
    assert_eq!(ids(&everyone.body).len(), 4);
    assert_eq!(total(&everyone), 4);
    let disabled = app
        .get("/api/v1/staff?enabled=false")
        .token(&s.recep)
        .send()
        .await
        .expect(StatusCode::OK);
    assert_eq!(ids(&disabled), [away]);

    for token in [&s.recep, &s.tech] {
        app.post("/api/v1/staff")
            .token(token)
            .json(&json!({ "username": "sneaky", "password": PASSWORD, "role": "Technician" }))
            .send()
            .await
            .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
    }
    for query in ["role=Manager", "enabled=maybe"] {
        app.get(&format!("/api/v1/staff?{query}"))
            .token(&s.recep)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
}

// --- BDD-023: work queues and paging (REQ-F-023) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_023_each_role_sees_its_work_queue(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let other_tech = app.create_staff(&s.admin, "lena.tech", "Technician").await;

    let received = received_order(&app, &s, "Standard").await;
    let awaiting = received_order(&app, &s, "Standard").await;
    diagnose(&app, &s, &awaiting).await;
    estimate(&app, &s, &awaiting, 5000, 5000).await;
    app.patch(&format!("/api/v1/orders/{}", awaiting.id))
        .token(&s.admin)
        .json(&json!({ "assigned_technician_id": other_tech }))
        .send()
        .await
        .expect(StatusCode::OK);
    let ready = in_progress_order(&app, &s, "Standard").await;
    repair(
        &app,
        &s,
        &ready,
        &json!({ "status": "Ready", "repair_checked": true }),
    )
    .await;
    let cancelled = received_order(&app, &s, "Standard").await;
    app.post(&cancelled.path("cancel"))
        .token(&s.recep)
        .send()
        .await
        .expect(StatusCode::OK);

    let list = |token: &str, query: &str| {
        let request = app.get(&format!("/api/v1/orders?{query}")).token(token);
        async move { request.send().await.expect(StatusCode::OK) }
    };
    let id = |order: &Order| order.id;

    assert_eq!(
        ids(&list(&s.recep, "awaiting_decision=true").await),
        [id(&awaiting)]
    );
    assert_eq!(ids(&list(&s.recep, "status=Ready").await), [id(&ready)]);
    assert_eq!(
        ids(&list(
            &s.tech,
            &format!("assigned_technician_id={}&open=true", s.tech_id)
        )
        .await),
        [id(&received), id(&ready)]
    );
    assert_eq!(
        ids(&list(
            &s.tech,
            &format!("assigned_technician_id={other_tech}&open=true")
        )
        .await),
        [id(&awaiting)]
    );
    assert_eq!(
        ids(&list(&s.admin, "open=true").await),
        [id(&received), id(&awaiting), id(&ready)]
    );

    let page = app
        .get("/api/v1/orders?open=true&limit=2&offset=0")
        .token(&s.admin)
        .send()
        .await;
    assert_eq!(page.status, StatusCode::OK);
    assert_eq!(total(&page), 3);
    assert_eq!(ids(&page.body), [id(&received), id(&awaiting)]);
    let page = app
        .get("/api/v1/orders?open=true&limit=2&offset=2")
        .token(&s.admin)
        .send()
        .await;
    assert_eq!(total(&page), 3);
    assert_eq!(ids(&page.body), [id(&ready)]);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn every_list_pages_and_counts(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let first = received_order(&app, &s, "Standard").await;
    let second = received_order(&app, &s, "Standard").await;

    for path in [
        "/api/v1/clients",
        "/api/v1/devices",
        "/api/v1/orders",
        "/api/v1/staff",
        "/api/v1/audit-logs",
    ] {
        let all = app.get(path).token(&s.admin).send().await;
        assert_eq!(all.status, StatusCode::OK, "{path}");
        let every = ids(&all.body);
        assert_eq!(total(&all), i64::try_from(every.len()).unwrap(), "{path}");
        assert!(every.len() >= 2, "{path}");

        let page = app
            .get(&format!("{path}?limit=1&offset=1"))
            .token(&s.admin)
            .send()
            .await;
        assert_eq!(page.status, StatusCode::OK, "{path}");
        assert_eq!(ids(&page.body), [every[1]], "{path}");
        assert_eq!(total(&page), total(&all), "{path}");

        for query in ["limit=0", "limit=101", "limit=x", "offset=-1"] {
            app.get(&format!("{path}?{query}"))
                .token(&s.admin)
                .send()
                .await
                .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        }
    }
    let past_the_end = app
        .get("/api/v1/orders?limit=10&offset=10")
        .token(&s.recep)
        .send()
        .await;
    assert_eq!(past_the_end.body, json!([]));
    assert_eq!(total(&past_the_end), 2);
    assert_eq!(
        ids(&app.get("/api/v1/orders").token(&s.recep).send().await.body),
        [first.id, second.id]
    );
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn queue_filters_reject_invalid_values(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    for query in [
        "open=yes",
        "awaiting_decision=1x",
        "assigned_technician_id=0",
    ] {
        app.get(&format!("/api/v1/orders?{query}"))
            .token(&s.recep)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
}
