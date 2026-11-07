//! Staff sign-in, role restrictions, staff accounts and their audit trail
//! (REQ-F-001, REQ-F-002, REQ-F-019, REQ-NF-002, REQ-NF-003; BDD-017, BDD-020).

mod common;

use axum::http::StatusCode;
use common::{ADMIN, PASSWORD, TestApp};
use serde_json::{Value, json};
use sqlx::PgPool;

const PROTECTED_READS: [&str; 2] = ["/api/v1/staff", "/api/v1/audit-logs"];

fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<_> = value
        .as_object()
        .expect("JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

fn assert_staff_account(value: &Value, username: &str, role: &str, enabled: bool) {
    assert_eq!(
        keys(value),
        ["enabled", "id", "role", "username"],
        "{value:#}"
    );
    assert!(value["id"].as_i64().is_some_and(|id| id >= 1), "{value:#}");
    assert_eq!(value["username"], username);
    assert_eq!(value["role"], role);
    assert_eq!(value["enabled"], enabled);
}

async fn staff_list(app: &TestApp, admin: &str) -> Value {
    app.get("/api/v1/staff")
        .token(admin)
        .send()
        .await
        .expect(StatusCode::OK)
}

async fn audit_log(app: &TestApp, token: &str, query: &str) -> Vec<Value> {
    let body = app
        .get(&format!("/api/v1/audit-logs{query}"))
        .token(token)
        .send()
        .await
        .expect(StatusCode::OK);
    body.as_array().expect("array of audit entries").clone()
}

async fn admin_id(app: &TestApp) -> i64 {
    let body = app
        .post("/api/v1/sessions")
        .json(&json!({ "username": ADMIN, "password": PASSWORD }))
        .send()
        .await
        .expect(StatusCode::CREATED);
    body["staff"]["id"].as_i64().expect("admin id")
}

// --- Sign-in required (REQ-F-001, BDD-017) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_017_request_without_token_is_denied(pool: PgPool) {
    let app = TestApp::new(pool);
    app.admin_token().await;
    for path in PROTECTED_READS {
        let body = app
            .get(path)
            .send()
            .await
            .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
        assert_eq!(
            keys(&body),
            ["code", "message"],
            "{path}: no record contents"
        );
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_017_unknown_token_is_denied(pool: PgPool) {
    let app = TestApp::new(pool);
    app.admin_token().await;
    for path in PROTECTED_READS {
        for token in ["garbage", "sess_recep_01", ""] {
            app.get(path)
                .token(token)
                .send()
                .await
                .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
        }
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_017_non_bearer_authorization_is_denied(pool: PgPool) {
    let app = TestApp::new(pool);
    let token = app.admin_token().await;
    app.get("/api/v1/staff")
        .header("authorization", &format!("Basic {token}"))
        .send()
        .await
        .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_017_expired_session_is_denied(pool: PgPool) {
    let app = TestApp::new(pool);
    let token = app.admin_token().await;
    staff_list(&app, &token).await;

    sqlx::query("UPDATE sessions SET expires_at = now() - interval '1 second'")
        .execute(&app.pool)
        .await
        .unwrap();

    for path in PROTECTED_READS {
        app.get(path)
            .token(&token)
            .send()
            .await
            .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
    }
}

// --- Sign-in (REQ-F-001, REQ-F-019) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn sign_in_returns_token_and_account(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    app.create_staff(&admin, "anna.recep", "Receptionist").await;

    let body = app
        .post("/api/v1/sessions")
        .json(&json!({ "username": "anna.recep", "password": PASSWORD }))
        .send()
        .await
        .expect(StatusCode::CREATED);

    assert_eq!(keys(&body), ["staff", "token"]);
    let token = body["token"].as_str().expect("token string");
    assert_ne!(token, "");
    assert_staff_account(&body["staff"], "anna.recep", "Receptionist", true);

    // The token works for a permitted action.
    app.get("/api/v1/audit-logs")
        .token(token)
        .send()
        .await
        .expect(StatusCode::OK);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn every_sign_in_issues_a_distinct_token(pool: PgPool) {
    let app = TestApp::new(pool);
    let first = app.admin_token().await;
    let second = app.sign_in(ADMIN, PASSWORD).await;
    assert_ne!(first, second);
    staff_list(&app, &first).await;
    staff_list(&app, &second).await;
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn wrong_password_and_unknown_user_are_indistinguishable(pool: PgPool) {
    let app = TestApp::new(pool);
    app.admin_token().await;

    let wrong_password = app
        .post("/api/v1/sessions")
        .json(&json!({ "username": ADMIN, "password": "not-the-password" }))
        .send()
        .await
        .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
    let unknown_user = app
        .post("/api/v1/sessions")
        .json(&json!({ "username": "nobody", "password": PASSWORD }))
        .send()
        .await
        .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");

    assert_eq!(wrong_password, unknown_user);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn sign_in_rejects_malformed_body(pool: PgPool) {
    let app = TestApp::new(pool);
    app.admin_token().await;
    for body in [
        json!({ "username": ADMIN }),
        json!({ "password": PASSWORD }),
        json!({ "username": "", "password": PASSWORD }),
        json!({ "username": ADMIN, "password": "" }),
        json!({ "username": ADMIN, "password": PASSWORD, "role": "Administrator" }),
    ] {
        app.post("/api/v1/sessions")
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
    app.post("/api/v1/sessions")
        .raw_json("{not json")
        .send()
        .await
        .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
}

// --- Disabling accounts (REQ-F-019) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn disabled_account_cannot_sign_in_and_loses_its_session(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let (tech_id, tech) = app.staff_token(&admin, "max.tech", "Technician").await;
    app.get("/api/v1/audit-logs")
        .token(&tech)
        .send()
        .await
        .expect(StatusCode::OK);

    let body = app
        .patch(&format!("/api/v1/staff/{tech_id}"))
        .token(&admin)
        .json(&json!({ "enabled": false }))
        .send()
        .await
        .expect(StatusCode::OK);
    assert_staff_account(&body, "max.tech", "Technician", false);

    app.get("/api/v1/audit-logs")
        .token(&tech)
        .send()
        .await
        .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");

    app.post("/api/v1/sessions")
        .json(&json!({ "username": "max.tech", "password": PASSWORD }))
        .send()
        .await
        .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");

    // A wrong password still says nothing about the account's state.
    app.post("/api/v1/sessions")
        .json(&json!({ "username": "max.tech", "password": "not-the-password" }))
        .send()
        .await
        .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn re_enabled_account_can_sign_in_again(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let tech_id = app.create_staff(&admin, "max.tech", "Technician").await;
    let path = format!("/api/v1/staff/{tech_id}");

    for enabled in [false, true] {
        app.patch(&path)
            .token(&admin)
            .json(&json!({ "enabled": enabled }))
            .send()
            .await
            .expect(StatusCode::OK);
    }
    app.sign_in("max.tech", PASSWORD).await;
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn disabling_one_account_keeps_other_sessions(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let (tech_id, _) = app.staff_token(&admin, "max.tech", "Technician").await;
    let (_, recep) = app.staff_token(&admin, "anna.recep", "Receptionist").await;

    app.patch(&format!("/api/v1/staff/{tech_id}"))
        .token(&admin)
        .json(&json!({ "enabled": false }))
        .send()
        .await
        .expect(StatusCode::OK);

    app.get("/api/v1/audit-logs")
        .token(&recep)
        .send()
        .await
        .expect(StatusCode::OK);
}

// --- Account management by Administrator (REQ-F-019) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn administrator_creates_an_account_in_each_role(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;

    for (username, role) in [
        ("anna.recep", "Receptionist"),
        ("max.tech", "Technician"),
        ("olga.admin", "Administrator"),
    ] {
        let body = app
            .post("/api/v1/staff")
            .token(&admin)
            .json(&json!({ "username": username, "password": PASSWORD, "role": role }))
            .send()
            .await
            .expect(StatusCode::CREATED);
        assert_staff_account(&body, username, role, true);

        let session = app
            .post("/api/v1/sessions")
            .json(&json!({ "username": username, "password": PASSWORD }))
            .send()
            .await
            .expect(StatusCode::CREATED);
        assert_eq!(session["staff"], body);
    }

    let list = staff_list(&app, &admin).await;
    let list = list.as_array().expect("array");
    assert_eq!(list.len(), 4);
    for account in list {
        assert_eq!(keys(account), ["enabled", "id", "role", "username"]);
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn created_administrator_can_manage_staff(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let (_, second_admin) = app.staff_token(&admin, "olga.admin", "Administrator").await;
    app.create_staff(&second_admin, "max.tech", "Technician")
        .await;
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn username_is_trimmed(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let body = app
        .post("/api/v1/staff")
        .token(&admin)
        .json(&json!({ "username": "  max.tech ", "password": PASSWORD, "role": "Technician" }))
        .send()
        .await
        .expect(StatusCode::CREATED);
    assert_eq!(body["username"], "max.tech");
    app.sign_in("max.tech", PASSWORD).await;
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn duplicate_username_conflicts(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    app.create_staff(&admin, "max.tech", "Technician").await;
    let before = staff_list(&app, &admin).await;

    app.post("/api/v1/staff")
        .token(&admin)
        .json(&json!({ "username": "max.tech", "password": PASSWORD, "role": "Receptionist" }))
        .send()
        .await
        .expect_error(StatusCode::CONFLICT, "CONFLICT");

    assert_eq!(staff_list(&app, &admin).await, before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn invalid_staff_account_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let audit_before = audit_log(&app, &admin, "").await;

    for body in [
        json!({ "username": "max.tech", "password": "1234567", "role": "Technician" }),
        json!({ "username": "   ", "password": PASSWORD, "role": "Technician" }),
        json!({ "username": "max.tech", "password": PASSWORD, "role": "Manager" }),
        json!({ "username": "max.tech", "password": PASSWORD }),
        json!({ "username": "max.tech", "role": "Technician" }),
        json!({ "username": "max.tech", "password": PASSWORD, "role": "Technician", "enabled": false }),
        json!({ "username": "max.tech", "password": PASSWORD, "role": "Technician", "password_hash": "x" }),
    ] {
        app.post("/api/v1/staff")
            .token(&admin)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }

    let short = app
        .post("/api/v1/staff")
        .token(&admin)
        .json(&json!({ "username": "max.tech", "password": "1234567", "role": "Technician" }))
        .send()
        .await
        .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    assert_eq!(short["details"]["field"], "password");

    assert_eq!(staff_list(&app, &admin).await.as_array().unwrap().len(), 1);
    assert_eq!(audit_log(&app, &admin, "").await, audit_before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn invalid_staff_update_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let tech_id = app.create_staff(&admin, "max.tech", "Technician").await;
    let path = format!("/api/v1/staff/{tech_id}");
    let before = staff_list(&app, &admin).await;

    for body in [
        json!({}),
        json!({ "role": "Manager" }),
        json!({ "enabled": "no" }),
        json!({ "username": "renamed" }),
        json!({ "role": "Receptionist", "password": "new-password-123" }),
    ] {
        app.patch(&path)
            .token(&admin)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
    assert_eq!(staff_list(&app, &admin).await, before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn update_of_unknown_staff_account_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    app.patch("/api/v1/staff/999999")
        .token(&admin)
        .json(&json!({ "enabled": false }))
        .send()
        .await
        .expect_error(StatusCode::NOT_FOUND, "NOT_FOUND");
}

// --- Last enabled Administrator (REQ-F-020, BDD-021) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn last_enabled_administrator_cannot_demote_or_disable(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let id = admin_id(&app).await;
    let audit_before: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    let sessions_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM sessions WHERE staff_id = $1")
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap();

    for change in [
        json!({ "role": "Receptionist" }),
        json!({ "enabled": false }),
        json!({ "role": "Receptionist", "enabled": false }),
    ] {
        app.patch(&format!("/api/v1/staff/{id}"))
            .token(&admin)
            .json(&change)
            .send()
            .await
            .expect_error(StatusCode::CONFLICT, "CONFLICT");
    }

    let row: (String, bool) =
        sqlx::query_as("SELECT role, enabled FROM staff_accounts WHERE id = $1")
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row, ("Administrator".to_owned(), true));
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions WHERE staff_id = $1")
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(sessions, sessions_before);
    let audit_after: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(audit_after, audit_before);
    staff_list(&app, &admin).await;
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn disabled_administrator_does_not_satisfy_last_admin_guard(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let id = admin_id(&app).await;
    let second = app
        .create_staff(&admin, "second.admin", "Administrator")
        .await;
    app.patch(&format!("/api/v1/staff/{second}"))
        .token(&admin)
        .json(&json!({ "enabled": false }))
        .send()
        .await
        .expect(StatusCode::OK);

    app.patch(&format!("/api/v1/staff/{id}"))
        .token(&admin)
        .json(&json!({ "enabled": false }))
        .send()
        .await
        .expect_error(StatusCode::CONFLICT, "CONFLICT");
    staff_list(&app, &admin).await;
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn another_enabled_administrator_allows_demotion(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let id = admin_id(&app).await;
    app.create_staff(&admin, "second.admin", "Administrator")
        .await;

    app.patch(&format!("/api/v1/staff/{id}"))
        .token(&admin)
        .json(&json!({ "role": "Receptionist" }))
        .send()
        .await
        .expect(StatusCode::OK);
    let row: (String, bool) =
        sqlx::query_as("SELECT role, enabled FROM staff_accounts WHERE id = $1")
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row, ("Receptionist".to_owned(), true));
    app.get("/api/v1/staff")
        .token(&admin)
        .send()
        .await
        .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
    assert_eq!(
        audit_log(
            &app,
            &admin,
            &format!("?record_type=StaffAccount&record_id={id}")
        )
        .await
        .len(),
        5
    );
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn promotion_restores_administrator_access(pool: PgPool) {
    use fixops_domain::id::StaffId;
    use fixops_domain::staff::{Actor, Role};

    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let admin_id = admin_id(&app).await;
    let target = app
        .create_staff(&admin, "recovery.admin", "Receptionist")
        .await;
    sqlx::query("UPDATE staff_accounts SET enabled = false WHERE id = $1")
        .bind(admin_id)
        .execute(&app.pool)
        .await
        .unwrap();

    app.app()
        .update_staff(
            Actor {
                id: StaffId(admin_id),
                role: Role::Administrator,
            },
            StaffId(target),
            Some(Role::Administrator),
            None,
        )
        .await
        .expect("promotion is allowed when there are no enabled Administrators");

    let enabled_admins: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM staff_accounts WHERE role = 'Administrator' AND enabled",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(enabled_admins, 1);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn concurrent_last_administrator_demotions_leave_one_enabled(pool: PgPool) {
    let app = TestApp::new(pool);
    let first_token = app.admin_token().await;
    let first_id = admin_id(&app).await;
    let second_id = app
        .create_staff(&first_token, "second.admin", "Administrator")
        .await;
    let second_token = app.sign_in("second.admin", PASSWORD).await;

    let first = app
        .patch(&format!("/api/v1/staff/{first_id}"))
        .token(&first_token)
        .json(&json!({ "enabled": false }))
        .send();
    let second = app
        .patch(&format!("/api/v1/staff/{second_id}"))
        .token(&second_token)
        .json(&json!({ "enabled": false }))
        .send();
    let (first, second) = tokio::join!(first, second);
    let statuses = [first.status, second.status];
    assert_eq!(
        statuses.iter().filter(|s| **s == StatusCode::OK).count(),
        1,
        "{statuses:?}"
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|s| **s == StatusCode::CONFLICT)
            .count(),
        1,
        "{statuses:?}"
    );

    let enabled_admins: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM staff_accounts WHERE role = 'Administrator' AND enabled",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(enabled_admins, 1);
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE record_type = 'StaffAccount' AND field = 'enabled' AND new_value = 'false'")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(audits, 1);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn staff_id_must_be_a_positive_integer(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    for id in ["0", "-1", "abc"] {
        app.patch(&format!("/api/v1/staff/{id}"))
            .token(&admin)
            .json(&json!({ "enabled": false }))
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
}

// --- Role restrictions (REQ-F-002, REQ-NF-002) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn receptionist_and_technician_cannot_manage_staff(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let target = app.create_staff(&admin, "target.tech", "Technician").await;
    let (_, recep) = app.staff_token(&admin, "anna.recep", "Receptionist").await;
    let (_, tech) = app.staff_token(&admin, "max.tech", "Technician").await;
    let staff_before = staff_list(&app, &admin).await;
    let audit_before = audit_log(&app, &admin, "").await;

    for token in [&recep, &tech] {
        app.get("/api/v1/staff")
            .token(token)
            .send()
            .await
            .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
        app.post("/api/v1/staff")
            .token(token)
            .json(&json!({ "username": "sneaky", "password": PASSWORD, "role": "Administrator" }))
            .send()
            .await
            .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
        for body in [
            json!({ "role": "Administrator" }),
            json!({ "enabled": false }),
        ] {
            app.patch(&format!("/api/v1/staff/{target}"))
                .token(token)
                .json(&body)
                .send()
                .await
                .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
        }
    }

    assert_eq!(staff_list(&app, &admin).await, staff_before);
    assert_eq!(audit_log(&app, &admin, "").await, audit_before);
    app.sign_in("target.tech", PASSWORD).await;
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn role_change_applies_to_the_existing_session(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let (tech_id, tech) = app.staff_token(&admin, "max.tech", "Technician").await;
    let path = format!("/api/v1/staff/{tech_id}");

    app.get("/api/v1/staff")
        .token(&tech)
        .send()
        .await
        .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");

    let body = app
        .patch(&path)
        .token(&admin)
        .json(&json!({ "role": "Administrator" }))
        .send()
        .await
        .expect(StatusCode::OK);
    assert_staff_account(&body, "max.tech", "Administrator", true);
    staff_list(&app, &tech).await;

    app.patch(&path)
        .token(&admin)
        .json(&json!({ "role": "Receptionist" }))
        .send()
        .await
        .expect(StatusCode::OK);
    app.get("/api/v1/staff")
        .token(&tech)
        .send()
        .await
        .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
}

// --- Audit trail (REQ-NF-002, REQ-NF-003) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn staff_creation_is_audited_without_the_password(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let admin_id = admin_id(&app).await;
    let tech_id = app.create_staff(&admin, "max.tech", "Technician").await;

    let entries = audit_log(
        &app,
        &admin,
        &format!("?record_type=StaffAccount&record_id={tech_id}"),
    )
    .await;

    let fields: Vec<_> = entries
        .iter()
        .map(|e| e["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields.len(), 4, "{entries:#?}");
    for field in ["username", "role", "enabled", "password"] {
        assert!(fields.contains(&field), "{field} not audited: {entries:#?}");
    }
    for entry in &entries {
        assert_eq!(
            keys(entry),
            [
                "actor_id",
                "field",
                "id",
                "new_value",
                "previous_value",
                "record_id",
                "record_type",
                "recorded_at"
            ]
        );
        assert_eq!(entry["actor_id"], admin_id);
        assert_eq!(entry["record_type"], "StaffAccount");
        assert_eq!(entry["record_id"], tech_id);
        assert_eq!(entry["previous_value"], Value::Null);
        assert!(
            entry["recorded_at"]
                .as_str()
                .is_some_and(|t| t.contains('T'))
        );
        let expected = match entry["field"].as_str().unwrap() {
            "username" => json!("max.tech"),
            "role" => json!("Technician"),
            "enabled" => json!("true"),
            _ => Value::Null,
        };
        assert_eq!(entry["new_value"], expected, "{entry:#}");
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn staff_updates_are_audited_in_order(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let admin_id = admin_id(&app).await;
    let tech_id = app.create_staff(&admin, "max.tech", "Technician").await;
    let path = format!("/api/v1/staff/{tech_id}");

    for body in [
        json!({ "role": "Receptionist" }),
        json!({ "enabled": false }),
        // Unchanged values write no entry.
        json!({ "role": "Receptionist", "enabled": false }),
    ] {
        app.patch(&path)
            .token(&admin)
            .json(&body)
            .send()
            .await
            .expect(StatusCode::OK);
    }

    let entries = audit_log(
        &app,
        &admin,
        &format!("?record_type=StaffAccount&record_id={tech_id}"),
    )
    .await;
    assert_eq!(entries.len(), 6, "{entries:#?}");
    let updates: Vec<_> = entries[4..]
        .iter()
        .map(|e| {
            (
                e["field"].clone(),
                e["previous_value"].clone(),
                e["new_value"].clone(),
                e["actor_id"].clone(),
            )
        })
        .collect();
    assert_eq!(
        updates,
        [
            (
                json!("role"),
                json!("Technician"),
                json!("Receptionist"),
                json!(admin_id)
            ),
            (
                json!("enabled"),
                json!("true"),
                json!("false"),
                json!(admin_id)
            ),
        ]
    );
    let ids: Vec<_> = entries.iter().map(|e| e["id"].as_i64().unwrap()).collect();
    assert!(ids.is_sorted(), "chronological order: {ids:?}");
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn no_password_or_hash_is_ever_returned(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let tech_id = app.create_staff(&admin, "max.tech", "Technician").await;
    app.patch(&format!("/api/v1/staff/{tech_id}"))
        .token(&admin)
        .json(&json!({ "enabled": false }))
        .send()
        .await
        .expect(StatusCode::OK);

    let staff = staff_list(&app, &admin).await.to_string();
    let audit = Value::Array(audit_log(&app, &admin, "").await).to_string();
    for text in [staff, audit] {
        assert!(!text.contains(PASSWORD), "{text}");
        assert!(!text.contains("argon2"), "{text}");
        assert!(!text.contains("password_hash"), "{text}");
    }

    let hashes: Vec<String> = sqlx::query_scalar("SELECT password_hash FROM staff_accounts")
        .fetch_all(&app.pool)
        .await
        .unwrap();
    for hash in hashes {
        assert_ne!(hash, PASSWORD, "passwords are stored hashed");
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn every_role_reads_the_audit_log(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let (_, recep) = app.staff_token(&admin, "anna.recep", "Receptionist").await;
    let (_, tech) = app.staff_token(&admin, "max.tech", "Technician").await;

    let expected = audit_log(&app, &admin, "").await;
    assert_ne!(expected, Vec::<Value>::new());
    for token in [&recep, &tech] {
        assert_eq!(audit_log(&app, token, "").await, expected);
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn audit_log_filters_by_record(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let recep_id = app.create_staff(&admin, "anna.recep", "Receptionist").await;
    let tech_id = app.create_staff(&admin, "max.tech", "Technician").await;

    for id in [recep_id, tech_id] {
        let entries = audit_log(&app, &admin, &format!("?record_id={id}")).await;
        assert_ne!(entries, Vec::<Value>::new());
        assert!(entries.iter().all(|e| e["record_id"] == id), "{entries:#?}");
    }

    let staff = audit_log(&app, &admin, "?record_type=StaffAccount").await;
    assert!(staff.iter().all(|e| e["record_type"] == "StaffAccount"));
    assert_eq!(
        audit_log(&app, &admin, "?record_type=Client").await,
        Vec::<Value>::new()
    );
    assert_eq!(
        audit_log(&app, &admin, "?record_type=StaffAccount&record_id=999999").await,
        Vec::<Value>::new()
    );
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn audit_log_rejects_invalid_filters(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    for query in [
        "?record_type=Invoice",
        "?record_id=0",
        "?record_id=-3",
        "?record_id=abc",
    ] {
        app.get(&format!("/api/v1/audit-logs{query}"))
            .token(&admin)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
}

// --- First Administrator ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bootstrap_runs_only_while_no_staff_exist(pool: PgPool) {
    use fixops_domain::staff::{Password, Username};

    let app = TestApp::new(pool);
    let admin = app.admin_token().await;

    let again = app
        .app()
        .bootstrap_admin(
            Username::parse("second.admin").unwrap(),
            Password::parse(PASSWORD.to_owned()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(again, None);

    let list = staff_list(&app, &admin).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_staff_account(&list[0], ADMIN, "Administrator", true);

    // The bootstrap account is attributed to itself in the audit log.
    let entries = audit_log(&app, &admin, "?record_type=StaffAccount").await;
    assert_eq!(entries.len(), 4);
    assert!(entries.iter().all(|e| e["actor_id"] == list[0]["id"]));
}

// --- Check order: 401 → 403 → 400 → 404/409 (ADD-002 §2, §5) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn denied_roles_get_403_whatever_the_body(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let target = app.create_staff(&admin, "target.tech", "Technician").await;
    let (_, recep) = app.staff_token(&admin, "anna.recep", "Receptionist").await;
    let (_, tech) = app.staff_token(&admin, "max.tech", "Technician").await;
    let staff_before = staff_list(&app, &admin).await;
    let audit_before = audit_log(&app, &admin, "").await;
    let target_path = format!("/api/v1/staff/{target}");

    for token in [&recep, &tech] {
        for body in [
            json!({ "username": "sneaky", "password": "1234567", "role": "Technician" }),
            json!({ "username": "sneaky", "password": PASSWORD, "role": "Manager" }),
            json!({ "username": "", "password": PASSWORD, "role": "Technician" }),
            json!({ "username": "sneaky", "password": PASSWORD, "role": "Technician", "x": 1 }),
            json!({ "username": "sneaky" }),
        ] {
            app.post("/api/v1/staff")
                .token(token)
                .json(&body)
                .send()
                .await
                .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
        }
        app.post("/api/v1/staff")
            .token(token)
            .raw_json("{not json")
            .send()
            .await
            .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");

        for body in [
            json!({}),
            json!({ "role": "Manager" }),
            json!({ "enabled": "no" }),
            json!({ "role": null, "enabled": false }),
            json!({ "username": "renamed" }),
        ] {
            app.patch(&target_path)
                .token(token)
                .json(&body)
                .send()
                .await
                .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
        }
        app.patch(&target_path)
            .token(token)
            .raw_json("{not json")
            .send()
            .await
            .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
        for id in ["0", "abc", "999999"] {
            app.patch(&format!("/api/v1/staff/{id}"))
                .token(token)
                .json(&json!({ "enabled": false }))
                .send()
                .await
                .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
        }
    }

    assert_eq!(staff_list(&app, &admin).await, staff_before);
    assert_eq!(audit_log(&app, &admin, "").await, audit_before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn missing_session_gets_401_whatever_the_body(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let target = app.create_staff(&admin, "target.tech", "Technician").await;
    let staff_before = staff_list(&app, &admin).await;

    for request in [
        app.post("/api/v1/staff").raw_json("{not json"),
        app.post("/api/v1/staff")
            .json(&json!({ "username": "x", "password": "1", "role": "Manager" })),
        app.patch(&format!("/api/v1/staff/{target}"))
            .raw_json("{not json"),
        app.patch("/api/v1/staff/0")
            .json(&json!({ "enabled": false })),
    ] {
        request
            .send()
            .await
            .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
    }
    assert_eq!(staff_list(&app, &admin).await, staff_before);
}

// --- Non-nullable fields (contract StaffUpdateRequest) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn null_fields_in_staff_update_are_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let tech_id = app.create_staff(&admin, "max.tech", "Technician").await;
    let path = format!("/api/v1/staff/{tech_id}");
    let staff_before = staff_list(&app, &admin).await;
    let audit_before = audit_log(&app, &admin, "").await;

    for body in [
        json!({ "role": null, "enabled": false }),
        json!({ "role": "Receptionist", "enabled": null }),
        json!({ "role": null }),
    ] {
        app.patch(&path)
            .token(&admin)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }

    assert_eq!(staff_list(&app, &admin).await, staff_before);
    assert_eq!(audit_log(&app, &admin, "").await, audit_before);
}

// --- Audit log is append-only (REQ-NF-003) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn audit_log_cannot_be_changed_deleted_or_truncated(pool: PgPool) {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let before = audit_log(&app, &admin, "").await;
    assert_ne!(before, Vec::<Value>::new());

    for statement in [
        "UPDATE audit_log SET new_value = 'x'",
        "DELETE FROM audit_log",
        "TRUNCATE audit_log",
    ] {
        let err = sqlx::query(statement)
            .execute(&app.pool)
            .await
            .expect_err(statement);
        assert!(
            err.to_string().contains("append-only"),
            "{statement}: {err}"
        );
    }
    assert_eq!(audit_log(&app, &admin, "").await, before);
}
