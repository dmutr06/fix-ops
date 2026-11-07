//! Client, device and repair-order intake, loyalty tiers and their audit trail
//! (REQ-F-001–005, REQ-F-017, REQ-NF-002, REQ-NF-003; BDD-001, BDD-002, BDD-011, BDD-015, BDD-017).

mod common;

use axum::http::{Method, StatusCode};
use common::{ADMIN, PASSWORD, TestApp};
use serde_json::{Value, json};
use sqlx::PgPool;

const CLIENT_KEYS: [&str; 5] = ["email", "id", "loyalty_tier", "name", "phone"];
const DEVICE_KEYS: [&str; 5] = [
    "client_id",
    "device_type",
    "id",
    "reported_symptoms",
    "serial_number",
];
const ORDER_KEYS: [&str; 16] = [
    "assigned_technician_id",
    "client_decision",
    "client_id",
    "completed_labor_cents",
    "device_id",
    "diagnostic_findings",
    "estimate",
    "id",
    "labor_discount_cents",
    "payment",
    "pickup",
    "repair_checked",
    "settlement_cents",
    "status",
    "ticket_number",
    "used_parts_cents",
];

/// Signed-in staff: one account per role.
struct Staff {
    admin: String,
    admin_id: i64,
    recep: String,
    recep_id: i64,
    tech: String,
    tech_id: i64,
}

async fn staff(app: &TestApp) -> Staff {
    let admin = app.admin_token().await;
    let admin_id = app
        .post("/api/v1/sessions")
        .json(&json!({ "username": ADMIN, "password": PASSWORD }))
        .send()
        .await
        .expect(StatusCode::CREATED)["staff"]["id"]
        .as_i64()
        .expect("admin id");
    let (recep_id, recep) = app.staff_token(&admin, "anna.recep", "Receptionist").await;
    let (tech_id, tech) = app.staff_token(&admin, "max.tech", "Technician").await;
    Staff {
        admin,
        admin_id,
        recep,
        recep_id,
        tech,
        tech_id,
    }
}

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

fn id(value: &Value) -> i64 {
    value["id"].as_i64().expect("record id")
}

fn bob() -> Value {
    json!({ "name": "Bob Vance", "phone": "+1-555-0100", "email": "bob@vance.org" })
}

fn laptop(client_id: i64) -> Value {
    json!({
        "client_id": client_id,
        "device_type": "Laptop",
        "serial_number": "SN-THINKPAD-9001",
        "reported_symptoms": "No power on AC adapter"
    })
}

fn intake(ticket: &str, client_id: i64, device_id: i64, technician_id: i64) -> Value {
    json!({
        "ticket_number": ticket,
        "client_id": client_id,
        "device_id": device_id,
        "assigned_technician_id": technician_id
    })
}

async fn create(app: &TestApp, token: &str, path: &str, body: &Value) -> Value {
    app.post(&format!("/api/v1{path}"))
        .token(token)
        .json(body)
        .send()
        .await
        .expect(StatusCode::CREATED)
}

async fn create_client(app: &TestApp, token: &str, body: &Value) -> i64 {
    id(&create(app, token, "/clients", body).await)
}

async fn create_device(app: &TestApp, token: &str, body: &Value) -> i64 {
    id(&create(app, token, "/devices", body).await)
}

/// A client with one laptop, created by `token`. Returns `(client_id, device_id)`.
async fn client_with_laptop(app: &TestApp, token: &str) -> (i64, i64) {
    let client = create_client(app, token, &bob()).await;
    let device = create_device(app, token, &laptop(client)).await;
    (client, device)
}

async fn get(app: &TestApp, token: &str, path: &str) -> Value {
    app.get(&format!("/api/v1{path}"))
        .token(token)
        .send()
        .await
        .expect(StatusCode::OK)
}

async fn list(app: &TestApp, token: &str, path: &str) -> Vec<Value> {
    get(app, token, path)
        .await
        .as_array()
        .expect("JSON array")
        .clone()
}

async fn audit(app: &TestApp, token: &str, record_type: &str, record_id: i64) -> Vec<Value> {
    list(
        app,
        token,
        &format!("/audit-logs?record_type={record_type}&record_id={record_id}"),
    )
    .await
}

/// Everything a rejected request could have changed.
async fn snapshot(app: &TestApp, token: &str) -> [Vec<Value>; 4] {
    [
        list(app, token, "/clients").await,
        list(app, token, "/devices").await,
        list(app, token, "/orders").await,
        list(app, token, "/audit-logs").await,
    ]
}

/// `(field, previous_value, new_value, actor_id)` of each entry, in order.
fn changes(entries: &[Value]) -> Vec<(Value, Value, Value, Value)> {
    entries
        .iter()
        .map(|e| {
            (
                e["field"].clone(),
                e["previous_value"].clone(),
                e["new_value"].clone(),
                e["actor_id"].clone(),
            )
        })
        .collect()
}

#[track_caller]
fn assert_client(value: &Value, name: &str, phone: &str, email: Option<&str>, tier: &str) {
    assert_eq!(keys(value), CLIENT_KEYS, "{value:#}");
    assert!(value["id"].as_i64().is_some_and(|id| id >= 1), "{value:#}");
    assert_eq!(value["name"], name);
    assert_eq!(value["phone"], phone);
    assert_eq!(value["email"], json!(email));
    assert_eq!(value["loyalty_tier"], tier);
}

/// A new order in `Received` with nothing recorded, exactly as `RepairOrder` in the contract.
#[track_caller]
fn assert_received_order(value: &Value, ticket: &str, client: i64, device: i64, technician: i64) {
    assert_eq!(keys(value), ORDER_KEYS, "{value:#}");
    assert!(value["id"].as_i64().is_some_and(|id| id >= 1), "{value:#}");
    assert_eq!(value["ticket_number"], ticket);
    assert_eq!(value["client_id"], client);
    assert_eq!(value["device_id"], device);
    assert_eq!(value["assigned_technician_id"], technician);
    assert_eq!(value["status"], "Received");
    for absent in [
        "diagnostic_findings",
        "estimate",
        "client_decision",
        "payment",
        "pickup",
    ] {
        assert_eq!(value[absent], Value::Null, "{absent}: {value:#}");
    }
    // Integer cents, never floats (ADD-002 §4).
    for cents in [
        "used_parts_cents",
        "completed_labor_cents",
        "labor_discount_cents",
        "settlement_cents",
    ] {
        assert_eq!(value[cents], json!(0), "{cents}: {value:#}");
        assert!(value[cents].is_i64(), "{cents}: {value:#}");
    }
    assert_eq!(value["repair_checked"], false);
}

// --- BDD-001: intake with required records (REQ-F-003–005) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_001_receptionist_links_client_and_device_to_a_received_order(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;

    let client = create(&app, &s.recep, "/clients", &bob()).await;
    assert_client(
        &client,
        "Bob Vance",
        "+1-555-0100",
        Some("bob@vance.org"),
        "Standard",
    );
    let client_id = id(&client);

    let device = create(&app, &s.recep, "/devices", &laptop(client_id)).await;
    assert_eq!(keys(&device), DEVICE_KEYS, "{device:#}");
    assert_eq!(device["client_id"], client_id);
    assert_eq!(device["device_type"], "Laptop");
    assert_eq!(device["serial_number"], "SN-THINKPAD-9001");
    assert_eq!(device["reported_symptoms"], "No power on AC adapter");
    let device_id = id(&device);

    let order = create(
        &app,
        &s.recep,
        "/orders",
        &intake("FIX-2026-0001", client_id, device_id, s.tech_id),
    )
    .await;
    assert_received_order(&order, "FIX-2026-0001", client_id, device_id, s.tech_id);

    // Every record can be reopened as saved.
    assert_eq!(
        get(&app, &s.recep, &format!("/clients/{client_id}")).await,
        client
    );
    assert_eq!(
        get(&app, &s.recep, &format!("/devices/{device_id}")).await,
        device
    );
    assert_eq!(
        get(&app, &s.recep, &format!("/orders/{}", id(&order))).await,
        order
    );
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_001_administrator_can_run_intake(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (client, device) = client_with_laptop(&app, &s.admin).await;
    let order = create(
        &app,
        &s.admin,
        "/orders",
        &intake("FIX-2026-0002", client, device, s.tech_id),
    )
    .await;
    assert_received_order(&order, "FIX-2026-0002", client, device, s.tech_id);
}

// --- BDD-002: missing intake data (REQ-F-003–005) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_002_device_without_serial_number_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let client = create_client(&app, &s.recep, &bob()).await;
    let before = snapshot(&app, &s.admin).await;

    let mut missing = laptop(client);
    missing.as_object_mut().unwrap().remove("serial_number");
    let mut empty = laptop(client);
    empty["serial_number"] = json!("");
    let mut blank = laptop(client);
    blank["serial_number"] = json!("   ");
    let mut null = laptop(client);
    null["serial_number"] = Value::Null;

    for body in [&empty, &blank] {
        let err = app
            .post("/api/v1/devices")
            .token(&s.recep)
            .json(body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        assert_eq!(err["details"]["field"], "serial_number", "{body}");
    }
    for body in [&missing, &null] {
        app.post("/api/v1/devices")
            .token(&s.recep)
            .json(body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }

    // No incomplete record and no audit entry.
    assert_eq!(snapshot(&app, &s.admin).await, before);
    assert_eq!(list(&app, &s.admin, "/devices").await, Vec::<Value>::new());
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_002_duplicate_ticket_number_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (client, device) = client_with_laptop(&app, &s.recep).await;
    let first = create(
        &app,
        &s.recep,
        "/orders",
        &intake("FIX-2026-0001", client, device, s.tech_id),
    )
    .await;
    let before = snapshot(&app, &s.admin).await;

    for ticket in ["FIX-2026-0001", "  FIX-2026-0001 "] {
        let err = app
            .post("/api/v1/orders")
            .token(&s.recep)
            .json(&intake(ticket, client, device, s.tech_id))
            .send()
            .await
            .expect_error(StatusCode::CONFLICT, "CONFLICT");
        assert_eq!(err["details"]["ticket_number"], "FIX-2026-0001", "{err:#}");
    }

    // No duplicate record; the first order is unchanged.
    assert_eq!(snapshot(&app, &s.admin).await, before);
    assert_eq!(list(&app, &s.admin, "/orders").await, vec![first]);
}

// --- BDD-011, BDD-015: loyalty tier (REQ-F-017) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_011_technician_cannot_change_a_loyalty_tier(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (client, device) = client_with_laptop(&app, &s.recep).await;
    create(
        &app,
        &s.recep,
        "/orders",
        &intake("FIX-2026-0001", client, device, s.tech_id),
    )
    .await;
    let before = snapshot(&app, &s.admin).await;

    for token in [&s.tech, &s.recep] {
        app.put(&format!("/api/v1/clients/{client}/loyalty-tier"))
            .token(token)
            .json(&json!({ "loyalty_tier": "Gold" }))
            .send()
            .await
            .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
    }

    let reopened = get(&app, &s.tech, &format!("/clients/{client}")).await;
    assert_eq!(reopened["loyalty_tier"], "Standard");
    assert_eq!(snapshot(&app, &s.admin).await, before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_015_administrator_changes_the_tier_and_it_is_audited(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let client = create_client(&app, &s.recep, &bob()).await;
    let path = format!("/api/v1/clients/{client}/loyalty-tier");

    for tier in ["Silver", "Gold"] {
        let updated = app
            .put(&path)
            .token(&s.admin)
            .json(&json!({ "loyalty_tier": tier }))
            .send()
            .await
            .expect(StatusCode::OK);
        assert_client(
            &updated,
            "Bob Vance",
            "+1-555-0100",
            Some("bob@vance.org"),
            tier,
        );
    }
    // Setting the current tier again changes nothing and writes no entry.
    app.put(&path)
        .token(&s.admin)
        .json(&json!({ "loyalty_tier": "Gold" }))
        .send()
        .await
        .expect(StatusCode::OK);

    let reopened = get(&app, &s.recep, &format!("/clients/{client}")).await;
    assert_eq!(reopened["loyalty_tier"], "Gold");

    let tier_changes: Vec<_> = audit(&app, &s.admin, "Client", client)
        .await
        .into_iter()
        .filter(|e| e["field"] == "loyalty_tier" && !e["previous_value"].is_null())
        .collect();
    assert_eq!(
        changes(&tier_changes),
        [
            (
                json!("loyalty_tier"),
                json!("Standard"),
                json!("Silver"),
                json!(s.admin_id)
            ),
            (
                json!("loyalty_tier"),
                json!("Silver"),
                json!("Gold"),
                json!(s.admin_id)
            ),
        ]
    );
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn invalid_loyalty_tier_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let client = create_client(&app, &s.recep, &bob()).await;
    let before = snapshot(&app, &s.admin).await;

    for body in [
        json!({ "loyalty_tier": "Platinum" }),
        json!({ "loyalty_tier": "gold" }),
        json!({ "loyalty_tier": null }),
        json!({ "loyalty_tier": 2 }),
        json!({}),
        json!({ "loyalty_tier": "Gold", "name": "Rob" }),
    ] {
        app.put(&format!("/api/v1/clients/{client}/loyalty-tier"))
            .token(&s.admin)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
    assert_eq!(snapshot(&app, &s.admin).await, before);
}

// --- BDD-017: sign-in required (REQ-F-001) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn bdd_017_visitor_gets_no_client_device_or_order(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (client, device) = client_with_laptop(&app, &s.recep).await;
    let order = id(&create(
        &app,
        &s.recep,
        "/orders",
        &intake("FIX-2026-0001", client, device, s.tech_id),
    )
    .await);
    let before = snapshot(&app, &s.admin).await;

    let requests = [
        (Method::GET, "/api/v1/clients".to_owned()),
        (Method::GET, format!("/api/v1/clients/{client}")),
        (Method::GET, "/api/v1/clients?q=Bob".to_owned()),
        (Method::GET, "/api/v1/devices".to_owned()),
        (Method::GET, format!("/api/v1/devices/{device}")),
        (Method::GET, "/api/v1/orders".to_owned()),
        (Method::GET, format!("/api/v1/orders/{order}")),
        (Method::POST, "/api/v1/clients".to_owned()),
        (Method::PATCH, format!("/api/v1/clients/{client}")),
        (
            Method::PUT,
            format!("/api/v1/clients/{client}/loyalty-tier"),
        ),
        (Method::POST, "/api/v1/devices".to_owned()),
        (Method::PATCH, format!("/api/v1/devices/{device}")),
        (Method::POST, "/api/v1/orders".to_owned()),
    ];
    for (method, path) in requests {
        for token in [None, Some("garbage")] {
            let mut request = app.request(method.clone(), &path).json(&json!({
                "name": "Mallory",
                "loyalty_tier": "Gold",
                "phone": "+1-555-0666"
            }));
            if let Some(token) = token {
                request = request.token(token);
            }
            let body = request
                .send()
                .await
                .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
            assert_eq!(keys(&body), ["code", "message"], "{method} {path}");
            let text = body.to_string();
            for content in [
                "Bob Vance",
                "+1-555-0100",
                "SN-THINKPAD-9001",
                "FIX-2026-0001",
            ] {
                assert!(!text.contains(content), "{method} {path}: {text}");
            }
        }
    }
    assert_eq!(snapshot(&app, &s.admin).await, before);
}

// --- Role restrictions (REQ-F-002, REQ-NF-002, ADD-001, ADD-002) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn every_role_reads_and_searches_clients_devices_and_orders(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (client, device) = client_with_laptop(&app, &s.recep).await;
    let order = id(&create(
        &app,
        &s.recep,
        "/orders",
        &intake("FIX-2026-0001", client, device, s.tech_id),
    )
    .await);

    for token in [&s.recep, &s.tech, &s.admin] {
        for path in [
            "/clients".to_owned(),
            "/clients?q=bob".to_owned(),
            "/devices".to_owned(),
            format!("/devices?client_id={client}"),
            "/orders".to_owned(),
            "/orders?status=Received".to_owned(),
        ] {
            assert_eq!(list(&app, token, &path).await.len(), 1, "{path}");
        }
        assert_eq!(
            id(&get(&app, token, &format!("/clients/{client}")).await),
            client
        );
        assert_eq!(
            id(&get(&app, token, &format!("/devices/{device}")).await),
            device
        );
        assert_eq!(
            id(&get(&app, token, &format!("/orders/{order}")).await),
            order
        );
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn receptionist_and_administrator_maintain_clients_devices_and_orders(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;

    for (n, token) in [&s.recep, &s.admin].into_iter().enumerate() {
        let (client, device) = client_with_laptop(&app, token).await;
        app.patch(&format!("/api/v1/clients/{client}"))
            .token(token)
            .json(&json!({ "phone": "+1-555-0199" }))
            .send()
            .await
            .expect(StatusCode::OK);
        app.patch(&format!("/api/v1/devices/{device}"))
            .token(token)
            .json(&json!({ "reported_symptoms": "Battery LED blinks 3x" }))
            .send()
            .await
            .expect(StatusCode::OK);
        create(
            &app,
            token,
            "/orders",
            &intake(&format!("FIX-2026-000{n}"), client, device, s.tech_id),
        )
        .await;
    }
    assert_eq!(list(&app, &s.admin, "/orders").await.len(), 2);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn technician_cannot_maintain_clients_devices_or_orders(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (client, device) = client_with_laptop(&app, &s.recep).await;
    let before = snapshot(&app, &s.admin).await;
    let before_client = get(&app, &s.admin, &format!("/clients/{client}")).await;
    let before_device = get(&app, &s.admin, &format!("/devices/{device}")).await;

    let requests = [
        (Method::POST, "/api/v1/clients".to_owned(), bob()),
        (
            Method::PATCH,
            format!("/api/v1/clients/{client}"),
            json!({ "phone": "+1-555-0199" }),
        ),
        (
            Method::PUT,
            format!("/api/v1/clients/{client}/loyalty-tier"),
            json!({ "loyalty_tier": "Gold" }),
        ),
        (Method::POST, "/api/v1/devices".to_owned(), laptop(client)),
        (
            Method::PATCH,
            format!("/api/v1/devices/{device}"),
            json!({ "serial_number": "SN-OTHER" }),
        ),
        (
            Method::POST,
            "/api/v1/orders".to_owned(),
            intake("FIX-2026-0001", client, device, s.tech_id),
        ),
    ];
    for (method, path, body) in requests {
        app.request(method.clone(), &path)
            .token(&s.tech)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
    }

    assert_eq!(snapshot(&app, &s.admin).await, before);
    assert_eq!(
        get(&app, &s.admin, &format!("/clients/{client}")).await,
        before_client
    );
    assert_eq!(
        get(&app, &s.admin, &format!("/devices/{device}")).await,
        before_device
    );
}

/// Role is checked before the id and the body, so a denied role learns nothing about
/// validation or which records exist (architecture § Request flow).
#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn denied_roles_get_403_whatever_the_request(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let client = create_client(&app, &s.recep, &bob()).await;
    let before = snapshot(&app, &s.admin).await;

    let tech_requests = [
        (Method::POST, "/api/v1/clients".to_owned(), json!({})),
        (
            Method::POST,
            "/api/v1/clients".to_owned(),
            json!({ "name": "" }),
        ),
        (Method::PATCH, "/api/v1/clients/0".to_owned(), json!({})),
        (
            Method::PATCH,
            "/api/v1/clients/abc".to_owned(),
            json!({ "x": 1 }),
        ),
        (
            Method::PATCH,
            "/api/v1/clients/999999".to_owned(),
            json!({ "phone": "1" }),
        ),
        (
            Method::POST,
            "/api/v1/devices".to_owned(),
            json!({ "client_id": 0 }),
        ),
        (Method::POST, "/api/v1/devices".to_owned(), laptop(999_999)),
        (
            Method::PATCH,
            "/api/v1/devices/0".to_owned(),
            json!({ "serial_number": null }),
        ),
        (
            Method::POST,
            "/api/v1/orders".to_owned(),
            json!({ "ticket_number": "" }),
        ),
        (
            Method::POST,
            "/api/v1/orders".to_owned(),
            intake("FIX-1", 999_999, 999_999, 999_999),
        ),
    ];
    for (method, path, body) in tech_requests {
        app.request(method.clone(), &path)
            .token(&s.tech)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
    }

    for (path, body) in [
        (
            format!("/api/v1/clients/{client}/loyalty-tier"),
            json!({ "loyalty_tier": "Platinum" }),
        ),
        ("/api/v1/clients/0/loyalty-tier".to_owned(), json!({})),
        (
            "/api/v1/clients/abc/loyalty-tier".to_owned(),
            json!({ "loyalty_tier": null }),
        ),
        (
            "/api/v1/clients/999999/loyalty-tier".to_owned(),
            json!({ "loyalty_tier": "Gold" }),
        ),
    ] {
        for token in [&s.recep, &s.tech] {
            app.put(&path)
                .token(token)
                .json(&body)
                .send()
                .await
                .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");
        }
    }

    app.post("/api/v1/clients")
        .token(&s.tech)
        .raw_json("{not json")
        .send()
        .await
        .expect_error(StatusCode::FORBIDDEN, "FORBIDDEN");

    assert_eq!(snapshot(&app, &s.admin).await, before);
}

// --- Clients (REQ-F-003) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn client_requires_name_and_phone(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;

    for (field, value) in [
        ("name", json!("")),
        ("name", json!("   ")),
        ("phone", json!("")),
        ("phone", json!(" \t ")),
    ] {
        let mut body = bob();
        body[field] = value;
        let err = app
            .post("/api/v1/clients")
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        assert_eq!(err["details"]["field"], field, "{body}");
    }

    for body in [
        json!({ "phone": "+1-555-0100" }),
        json!({ "name": "Bob Vance" }),
        json!({ "name": null, "phone": "+1-555-0100" }),
        json!({ "name": "Bob Vance", "phone": null }),
        json!({ "name": 42, "phone": "+1-555-0100" }),
        json!({}),
    ] {
        app.post("/api/v1/clients")
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }

    assert_eq!(list(&app, &s.admin, "/clients").await, Vec::<Value>::new());
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn client_email_is_optional_and_nullable(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;

    for body in [
        json!({ "name": "Alice Smith", "phone": "+1-555-0200" }),
        json!({ "name": "Alice Smith", "phone": "+1-555-0200", "email": null }),
    ] {
        let client = create(&app, &s.recep, "/clients", &body).await;
        assert_client(&client, "Alice Smith", "+1-555-0200", None, "Standard");
        let reopened = get(&app, &s.recep, &format!("/clients/{}", id(&client))).await;
        assert_eq!(reopened, client);
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn client_values_are_trimmed(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let client = create(
        &app,
        &s.recep,
        "/clients",
        &json!({ "name": "  Bob Vance ", "phone": " +1-555-0100 ", "email": " bob@vance.org " }),
    )
    .await;
    assert_client(
        &client,
        "Bob Vance",
        "+1-555-0100",
        Some("bob@vance.org"),
        "Standard",
    );
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn invalid_client_email_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let client = create_client(&app, &s.recep, &bob()).await;
    let before = snapshot(&app, &s.admin).await;

    for email in [
        "",
        "bob",
        "bob@",
        "@vance.org",
        "bob@vance",
        "bob @vance.org",
    ] {
        let mut body = bob();
        body["email"] = json!(email);
        let err = app
            .post("/api/v1/clients")
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        assert_eq!(err["details"]["field"], "email", "{email:?}");

        let err = app
            .patch(&format!("/api/v1/clients/{client}"))
            .token(&s.recep)
            .json(&json!({ "email": email }))
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        assert_eq!(err["details"]["field"], "email", "{email:?}");
    }
    assert_eq!(snapshot(&app, &s.admin).await, before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn client_create_rejects_unknown_fields_and_malformed_json(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;

    for field in ["loyalty_tier", "id", "address"] {
        let mut body = bob();
        body[field] = json!("Gold");
        app.post("/api/v1/clients")
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
    for raw in ["{not json", "[]", "\"Bob\""] {
        app.post("/api/v1/clients")
            .token(&s.recep)
            .raw_json(raw)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
    assert_eq!(list(&app, &s.admin, "/clients").await, Vec::<Value>::new());
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn client_update_changes_only_the_given_fields(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let client = create_client(&app, &s.recep, &bob()).await;
    let path = format!("/api/v1/clients/{client}");

    let updated = app
        .patch(&path)
        .token(&s.recep)
        .json(&json!({ "phone": "+1-555-0199" }))
        .send()
        .await
        .expect(StatusCode::OK);
    assert_client(
        &updated,
        "Bob Vance",
        "+1-555-0199",
        Some("bob@vance.org"),
        "Standard",
    );

    let updated = app
        .patch(&path)
        .token(&s.recep)
        .json(&json!({ "name": "Robert Vance", "email": null }))
        .send()
        .await
        .expect(StatusCode::OK);
    assert_client(&updated, "Robert Vance", "+1-555-0199", None, "Standard");
    assert_eq!(
        get(&app, &s.tech, &format!("/clients/{client}")).await,
        updated
    );
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn client_update_rejects_invalid_values(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let client = create_client(&app, &s.recep, &bob()).await;
    let before = snapshot(&app, &s.admin).await;

    for body in [
        json!({}),
        json!({ "name": "" }),
        json!({ "name": "   " }),
        json!({ "phone": "" }),
        json!({ "name": null }),
        json!({ "phone": null }),
        json!({ "name": "Rob", "phone": null }),
        json!({ "loyalty_tier": "Gold" }),
        json!({ "id": 99 }),
    ] {
        app.patch(&format!("/api/v1/clients/{client}"))
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
    assert_eq!(snapshot(&app, &s.admin).await, before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn client_search_matches_name_or_phone_ignoring_case(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let bob = create_client(&app, &s.recep, &bob()).await;
    let alice = create_client(
        &app,
        &s.recep,
        &json!({ "name": "Alice Smith", "phone": "+1-555-0200" }),
    )
    .await;

    let ids = |found: Vec<Value>| {
        let mut ids: Vec<i64> = found.iter().map(id).collect();
        ids.sort_unstable();
        ids
    };
    for (q, expected) in [
        ("VANCE", vec![bob]),
        ("vance", vec![bob]),
        ("ance", vec![bob]),
        ("aLiCe", vec![alice]),
        ("0200", vec![alice]),
        ("%2B1-555-01", vec![bob]),
        ("555", vec![bob, alice]),
        ("i", vec![alice]),
        ("nobody", vec![]),
        ("%25", vec![]),
        ("_", vec![]),
    ] {
        let found = list(&app, &s.tech, &format!("/clients?q={q}")).await;
        assert_eq!(ids(found), expected, "q={q}");
    }
    let all = list(&app, &s.tech, "/clients").await;
    assert_eq!(ids(all.clone()), vec![bob, alice]);
    for client in &all {
        assert_eq!(keys(client), CLIENT_KEYS);
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn client_search_rejects_invalid_parameters(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    for query in ["?q=", "?name=Bob"] {
        app.get(&format!("/api/v1/clients{query}"))
            .token(&s.recep)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
}

// --- Devices (REQ-F-004) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn device_requires_type_serial_number_and_symptoms(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let client = create_client(&app, &s.recep, &bob()).await;

    for (field, value) in [
        ("device_type", json!("Phone")),
        ("device_type", json!("laptop")),
        ("serial_number", json!("")),
        ("reported_symptoms", json!("")),
        ("reported_symptoms", json!("  ")),
    ] {
        let mut body = laptop(client);
        body[field] = value;
        let err = app
            .post("/api/v1/devices")
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        assert_eq!(err["details"]["field"], field, "{body}");
    }

    for field in [
        "client_id",
        "device_type",
        "serial_number",
        "reported_symptoms",
    ] {
        let mut missing = laptop(client);
        missing.as_object_mut().unwrap().remove(field);
        let mut null = laptop(client);
        null[field] = Value::Null;
        for body in [missing, null] {
            app.post("/api/v1/devices")
                .token(&s.recep)
                .json(&body)
                .send()
                .await
                .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        }
    }

    for client_id in [json!(0), json!(-1), json!("1"), json!(1.5)] {
        let mut body = laptop(client);
        body["client_id"] = client_id;
        app.post("/api/v1/devices")
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }

    let mut extra = laptop(client);
    extra["id"] = json!(5);
    app.post("/api/v1/devices")
        .token(&s.recep)
        .json(&extra)
        .send()
        .await
        .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");

    assert_eq!(list(&app, &s.admin, "/devices").await, Vec::<Value>::new());
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn device_of_unknown_client_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let before = snapshot(&app, &s.admin).await;
    let err = app
        .post("/api/v1/devices")
        .token(&s.recep)
        .json(&laptop(999_999))
        .send()
        .await
        .expect_error(StatusCode::NOT_FOUND, "NOT_FOUND");
    assert_eq!(err["details"]["field"], "client_id", "{err:#}");
    assert_eq!(snapshot(&app, &s.admin).await, before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn device_update_changes_only_the_given_fields(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (client, device) = client_with_laptop(&app, &s.recep).await;
    let path = format!("/api/v1/devices/{device}");

    let updated = app
        .patch(&path)
        .token(&s.recep)
        .json(&json!({ "reported_symptoms": "No power on AC adapter; battery LED blinks 3x" }))
        .send()
        .await
        .expect(StatusCode::OK);
    assert_eq!(
        updated,
        json!({
            "id": device,
            "client_id": client,
            "device_type": "Laptop",
            "serial_number": "SN-THINKPAD-9001",
            "reported_symptoms": "No power on AC adapter; battery LED blinks 3x"
        })
    );

    let updated = app
        .patch(&path)
        .token(&s.recep)
        .json(&json!({ "device_type": "Desktop", "serial_number": " SN-TOWER-1 " }))
        .send()
        .await
        .expect(StatusCode::OK);
    assert_eq!(updated["device_type"], "Desktop");
    assert_eq!(updated["serial_number"], "SN-TOWER-1");
    assert_eq!(
        get(&app, &s.tech, &format!("/devices/{device}")).await,
        updated
    );
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn device_update_rejects_invalid_values(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (other_client, _) = client_with_laptop(&app, &s.recep).await;
    let (_, device) = client_with_laptop(&app, &s.recep).await;
    let before = snapshot(&app, &s.admin).await;

    for body in [
        json!({}),
        json!({ "serial_number": "" }),
        json!({ "serial_number": "  " }),
        json!({ "reported_symptoms": "" }),
        json!({ "device_type": "Tablet" }),
        json!({ "device_type": null }),
        json!({ "serial_number": null }),
        json!({ "reported_symptoms": null }),
        json!({ "client_id": other_client }),
    ] {
        app.patch(&format!("/api/v1/devices/{device}"))
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
    assert_eq!(snapshot(&app, &s.admin).await, before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn device_list_filters_by_client_and_serial_number(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (bob, thinkpad) = client_with_laptop(&app, &s.recep).await;
    let tower = create_device(
        &app,
        &s.recep,
        &json!({
            "client_id": bob,
            "device_type": "Desktop",
            "serial_number": "SN-TOWER-77",
            "reported_symptoms": "Fans loud"
        }),
    )
    .await;
    let alice = create_client(
        &app,
        &s.recep,
        &json!({ "name": "Alice Smith", "phone": "+1-555-0200" }),
    )
    .await;
    let server = create_device(
        &app,
        &s.recep,
        &json!({
            "client_id": alice,
            "device_type": "Server",
            "serial_number": "SN-THINKSYSTEM-1",
            "reported_symptoms": "RAID degraded"
        }),
    )
    .await;

    let ids = |found: Vec<Value>| {
        let mut ids: Vec<i64> = found.iter().map(id).collect();
        ids.sort_unstable();
        ids
    };
    for (query, expected) in [
        (String::new(), vec![thinkpad, tower, server]),
        (format!("?client_id={bob}"), vec![thinkpad, tower]),
        (format!("?client_id={alice}"), vec![server]),
        ("?client_id=999999".to_owned(), vec![]),
        ("?q=thinkpad".to_owned(), vec![thinkpad]),
        ("?q=THINK".to_owned(), vec![thinkpad, server]),
        ("?q=tower-77".to_owned(), vec![tower]),
        ("?q=nothing".to_owned(), vec![]),
        (format!("?client_id={bob}&q=think"), vec![thinkpad]),
        (format!("?client_id={alice}&q=tower"), vec![]),
    ] {
        let found = list(&app, &s.tech, &format!("/devices{query}")).await;
        for device in &found {
            assert_eq!(keys(device), DEVICE_KEYS);
        }
        assert_eq!(ids(found), expected, "{query}");
    }

    for query in [
        "?client_id=0",
        "?client_id=-1",
        "?client_id=abc",
        "?q=",
        "?serial=1",
    ] {
        app.get(&format!("/api/v1/devices{query}"))
            .token(&s.tech)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
}

// --- Repair orders (REQ-F-005) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn order_requires_a_ticket_number_and_every_link(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (client, device) = client_with_laptop(&app, &s.recep).await;
    let valid = intake("FIX-2026-0001", client, device, s.tech_id);

    for value in [json!(""), json!("   ")] {
        let mut body = valid.clone();
        body["ticket_number"] = value;
        let err = app
            .post("/api/v1/orders")
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        assert_eq!(err["details"]["field"], "ticket_number", "{body}");
    }

    for field in ["client_id", "device_id", "assigned_technician_id"] {
        for value in [json!(0), json!(-5)] {
            let mut body = valid.clone();
            body[field] = value;
            let err = app
                .post("/api/v1/orders")
                .token(&s.recep)
                .json(&body)
                .send()
                .await
                .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
            assert_eq!(err["details"]["field"], field, "{body}");
        }
    }

    for field in [
        "ticket_number",
        "client_id",
        "device_id",
        "assigned_technician_id",
    ] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(field);
        let mut null = valid.clone();
        null[field] = Value::Null;
        for body in [missing, null] {
            app.post("/api/v1/orders")
                .token(&s.recep)
                .json(&body)
                .send()
                .await
                .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
        }
    }

    let mut wrong_types = valid.clone();
    wrong_types["client_id"] = json!("1");
    let mut extra = valid.clone();
    extra["status"] = json!("Ready");
    let mut charge = valid.clone();
    charge["used_parts_cents"] = json!(100);
    for body in [wrong_types, extra, charge, json!({})] {
        app.post("/api/v1/orders")
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }

    assert_eq!(list(&app, &s.admin, "/orders").await, Vec::<Value>::new());
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn order_with_an_unknown_link_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (client, device) = client_with_laptop(&app, &s.recep).await;
    let before = snapshot(&app, &s.admin).await;

    for (field, body) in [
        (
            "client_id",
            intake("FIX-2026-0001", 999_999, device, s.tech_id),
        ),
        (
            "device_id",
            intake("FIX-2026-0001", client, 999_999, s.tech_id),
        ),
        (
            "assigned_technician_id",
            intake("FIX-2026-0001", client, device, 999_999),
        ),
    ] {
        let err = app
            .post("/api/v1/orders")
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect_error(StatusCode::NOT_FOUND, "NOT_FOUND");
        assert_eq!(err["details"]["field"], field, "{err:#}");
    }
    assert_eq!(snapshot(&app, &s.admin).await, before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn order_for_a_device_of_another_client_conflicts(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (_, bobs_laptop) = client_with_laptop(&app, &s.recep).await;
    let alice = create_client(
        &app,
        &s.recep,
        &json!({ "name": "Alice Smith", "phone": "+1-555-0200" }),
    )
    .await;
    let before = snapshot(&app, &s.admin).await;

    let err = app
        .post("/api/v1/orders")
        .token(&s.recep)
        .json(&intake("FIX-2026-0001", alice, bobs_laptop, s.tech_id))
        .send()
        .await
        .expect_error(StatusCode::CONFLICT, "CONFLICT");
    assert_eq!(err["details"]["field"], "device_id", "{err:#}");
    assert_eq!(snapshot(&app, &s.admin).await, before);
}

/// REQ-F-005 and the glossary name an assigned Technician; see logs/test_gate_p2.md for why an
/// Administrator does not count.
#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn order_assignee_must_be_a_technician(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (client, device) = client_with_laptop(&app, &s.recep).await;
    let before = snapshot(&app, &s.admin).await;

    for assignee in [s.recep_id, s.admin_id] {
        let err = app
            .post("/api/v1/orders")
            .token(&s.recep)
            .json(&intake("FIX-2026-0001", client, device, assignee))
            .send()
            .await
            .expect_error(StatusCode::CONFLICT, "CONFLICT");
        assert_eq!(err["details"]["field"], "assigned_technician_id", "{err:#}");
    }
    assert_eq!(snapshot(&app, &s.admin).await, before);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn order_list_filters_by_status_client_and_ticket(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (bob, bobs_laptop) = client_with_laptop(&app, &s.recep).await;
    let (alice, alices_laptop) = client_with_laptop(&app, &s.recep).await;
    let mut orders = Vec::new();
    for (ticket, client, device) in [
        ("FIX-2026-0001", bob, bobs_laptop),
        ("FIX-2026-0002", bob, bobs_laptop),
        ("FIX-2026-0003", alice, alices_laptop),
    ] {
        let order = create(
            &app,
            &s.recep,
            "/orders",
            &intake(ticket, client, device, s.tech_id),
        )
        .await;
        orders.push(id(&order));
    }

    let ids = |found: Vec<Value>| {
        let mut ids: Vec<i64> = found.iter().map(id).collect();
        ids.sort_unstable();
        ids
    };
    for (query, expected) in [
        (String::new(), orders.clone()),
        ("?status=Received".to_owned(), orders.clone()),
        ("?status=Ready".to_owned(), vec![]),
        (format!("?client_id={bob}"), vec![orders[0], orders[1]]),
        (format!("?client_id={alice}"), vec![orders[2]]),
        ("?client_id=999999".to_owned(), vec![]),
        ("?ticket_number=FIX-2026-0002".to_owned(), vec![orders[1]]),
        ("?ticket_number=FIX-2026-9999".to_owned(), vec![]),
        (
            format!("?status=Received&client_id={alice}"),
            vec![orders[2]],
        ),
        (
            format!("?client_id={alice}&ticket_number=FIX-2026-0001"),
            vec![],
        ),
    ] {
        let found = list(&app, &s.tech, &format!("/orders{query}")).await;
        for order in &found {
            assert_eq!(keys(order), ORDER_KEYS);
        }
        assert_eq!(ids(found), expected, "{query}");
    }

    for query in [
        "?status=Done",
        "?status=received",
        "?status=",
        "?client_id=0",
        "?client_id=abc",
        "?ticket_number=",
        "?device_id=1",
    ] {
        app.get(&format!("/api/v1/orders{query}"))
            .token(&s.tech)
            .send()
            .await
            .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
    }
}

// --- Record ids (contract path parameters, minimum: 1) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn record_ids_must_be_positive_integers(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;

    for (method, template, param, body) in [
        (Method::GET, "/api/v1/clients/{}", "clientId", Value::Null),
        (
            Method::PATCH,
            "/api/v1/clients/{}",
            "clientId",
            json!({ "phone": "1" }),
        ),
        (
            Method::PUT,
            "/api/v1/clients/{}/loyalty-tier",
            "clientId",
            json!({ "loyalty_tier": "Gold" }),
        ),
        (Method::GET, "/api/v1/devices/{}", "deviceId", Value::Null),
        (
            Method::PATCH,
            "/api/v1/devices/{}",
            "deviceId",
            json!({ "serial_number": "SN-1" }),
        ),
        (Method::GET, "/api/v1/orders/{}", "orderId", Value::Null),
    ] {
        for raw in ["0", "-1", "abc", "1.5"] {
            let path = template.replace("{}", raw);
            let mut request = app.request(method.clone(), &path).token(&s.admin);
            if !body.is_null() {
                request = request.json(&body);
            }
            let err = request
                .send()
                .await
                .expect_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR");
            assert_eq!(err["details"]["field"], param, "{method} {path}");
        }
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn unknown_records_are_not_found(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let before = snapshot(&app, &s.admin).await;

    for (method, path, body) in [
        (Method::GET, "/api/v1/clients/999999", Value::Null),
        (
            Method::PATCH,
            "/api/v1/clients/999999",
            json!({ "phone": "+1-555-0199" }),
        ),
        (
            Method::PUT,
            "/api/v1/clients/999999/loyalty-tier",
            json!({ "loyalty_tier": "Gold" }),
        ),
        (Method::GET, "/api/v1/devices/999999", Value::Null),
        (
            Method::PATCH,
            "/api/v1/devices/999999",
            json!({ "serial_number": "SN-1" }),
        ),
        (Method::GET, "/api/v1/orders/999999", Value::Null),
    ] {
        let mut request = app.request(method.clone(), path).token(&s.admin);
        if !body.is_null() {
            request = request.json(&body);
        }
        request
            .send()
            .await
            .expect_error(StatusCode::NOT_FOUND, "NOT_FOUND");
    }
    assert_eq!(snapshot(&app, &s.admin).await, before);
}

// --- Audit trail (REQ-NF-003, AC-NF-003) ---

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn creation_is_audited_for_clients_devices_and_orders(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (client, device) = client_with_laptop(&app, &s.recep).await;
    let order = id(&create(
        &app,
        &s.recep,
        "/orders",
        &intake("FIX-2026-0001", client, device, s.tech_id),
    )
    .await);

    let client_entries = audit(&app, &s.tech, "Client", client).await;
    let device_entries = audit(&app, &s.tech, "Device", device).await;
    let order_entries = audit(&app, &s.tech, "RepairOrder", order).await;

    for (record_type, record_id, entries) in [
        ("Client", client, &client_entries),
        ("Device", device, &device_entries),
        ("RepairOrder", order, &order_entries),
    ] {
        assert_ne!(entries, &Vec::<Value>::new(), "{record_type}");
        for entry in entries {
            assert_eq!(entry["actor_id"], s.recep_id, "{entry:#}");
            assert_eq!(entry["record_type"], record_type);
            assert_eq!(entry["record_id"], record_id);
            assert_eq!(entry["previous_value"], Value::Null, "{entry:#}");
            assert!(
                entry["recorded_at"]
                    .as_str()
                    .is_some_and(|t| t.contains('T'))
            );
        }
    }

    let fields = |entries: &[Value]| {
        let mut pairs: Vec<(String, Value)> = entries
            .iter()
            .map(|e| {
                (
                    e["field"].as_str().unwrap().to_owned(),
                    e["new_value"].clone(),
                )
            })
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    };
    let pairs = |expected: &[(&str, Value)]| {
        let mut pairs: Vec<(String, Value)> = expected
            .iter()
            .map(|(f, v)| ((*f).to_owned(), v.clone()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    };
    assert_eq!(
        fields(&client_entries),
        pairs(&[
            ("name", json!("Bob Vance")),
            ("phone", json!("+1-555-0100")),
            ("email", json!("bob@vance.org")),
            ("loyalty_tier", json!("Standard")),
        ])
    );
    assert_eq!(
        fields(&device_entries),
        pairs(&[
            ("client_id", json!(client.to_string())),
            ("device_type", json!("Laptop")),
            ("serial_number", json!("SN-THINKPAD-9001")),
            ("reported_symptoms", json!("No power on AC adapter")),
        ])
    );
    let order_fields = fields(&order_entries);
    for expected in pairs(&[
        ("ticket_number", json!("FIX-2026-0001")),
        ("client_id", json!(client.to_string())),
        ("device_id", json!(device.to_string())),
        ("assigned_technician_id", json!(s.tech_id.to_string())),
        ("status", json!("Received")),
    ]) {
        assert!(
            order_fields.contains(&expected),
            "{expected:?} in {order_fields:?}"
        );
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn updates_are_audited_with_old_and_new_values(pool: PgPool) {
    let app = TestApp::new(pool);
    let s = staff(&app).await;
    let (client, device) = client_with_laptop(&app, &s.recep).await;
    let created_client = audit(&app, &s.admin, "Client", client).await.len();
    let created_device = audit(&app, &s.admin, "Device", device).await.len();

    for body in [
        json!({ "phone": "+1-555-0199" }),
        // Unchanged values write no entry.
        json!({ "phone": "+1-555-0199", "name": "Bob Vance" }),
        json!({ "email": null }),
    ] {
        app.patch(&format!("/api/v1/clients/{client}"))
            .token(&s.recep)
            .json(&body)
            .send()
            .await
            .expect(StatusCode::OK);
    }
    app.patch(&format!("/api/v1/devices/{device}"))
        .token(&s.admin)
        .json(&json!({ "reported_symptoms": "Battery LED blinks 3x", "device_type": "Laptop" }))
        .send()
        .await
        .expect(StatusCode::OK);

    let client_entries = audit(&app, &s.admin, "Client", client).await;
    assert_eq!(
        changes(&client_entries[created_client..]),
        [
            (
                json!("phone"),
                json!("+1-555-0100"),
                json!("+1-555-0199"),
                json!(s.recep_id)
            ),
            (
                json!("email"),
                json!("bob@vance.org"),
                Value::Null,
                json!(s.recep_id)
            ),
        ]
    );
    let device_entries = audit(&app, &s.admin, "Device", device).await;
    assert_eq!(
        changes(&device_entries[created_device..]),
        [(
            json!("reported_symptoms"),
            json!("No power on AC adapter"),
            json!("Battery LED blinks 3x"),
            json!(s.admin_id)
        )]
    );
}
