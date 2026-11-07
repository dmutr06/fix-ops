//! Repair-order workflow helpers shared by the diagnosis and repair tests: staff of every role,
//! orders at each lifecycle step, the contract `RepairOrder` body and audit read-backs.

use axum::http::StatusCode;
use serde_json::{Value, json};

use super::{ADMIN, PASSWORD, TestApp, TestResponse};

/// Keys of the contract `RepairOrder`, sorted.
pub const ORDER_KEYS: [&str; 16] = [
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

/// Findings from the contract examples.
pub const FINDINGS: &str = "Faulty USB-C power controller IC on mainboard";
/// Decision time from the contract examples.
pub const DECIDED_AT: &str = "2026-10-21T10:15:00Z";

/// Signed-in staff: one account per role.
pub struct Staff {
    pub admin: String,
    pub admin_id: i64,
    pub recep: String,
    pub recep_id: i64,
    pub tech: String,
    pub tech_id: i64,
}

pub async fn staff(app: &TestApp) -> Staff {
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

/// An order created through intake, with its links.
#[derive(Debug, Clone)]
pub struct Order {
    pub id: i64,
    pub ticket: String,
    pub client: i64,
    pub device: i64,
    pub technician: i64,
}

impl Order {
    /// `/api/v1/orders/{id}/{endpoint}`.
    pub fn path(&self, endpoint: &str) -> String {
        format!("/api/v1/orders/{}/{endpoint}", self.id)
    }

    /// The contract `RepairOrder` of this order as received, with `fields` replaced.
    pub fn expected(&self, fields: &Value) -> Value {
        let mut order = json!({
            "id": self.id,
            "ticket_number": self.ticket,
            "client_id": self.client,
            "device_id": self.device,
            "assigned_technician_id": self.technician,
            "status": "Received",
            "diagnostic_findings": null,
            "estimate": null,
            "client_decision": null,
            "used_parts_cents": 0,
            "completed_labor_cents": 0,
            "repair_checked": false,
            "labor_discount_cents": 0,
            "settlement_cents": 0,
            "payment": null,
            "pickup": null
        });
        for (key, value) in fields.as_object().expect("fields object") {
            assert!(order.get(key).is_some(), "unknown RepairOrder field {key}");
            order[key] = value.clone();
        }
        order
    }
}

pub fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<_> = value
        .as_object()
        .expect("JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

/// A new client at `tier` with one laptop and a `Received` order assigned to the Technician.
pub async fn received_order(app: &TestApp, s: &Staff, tier: &str) -> Order {
    let client = app
        .post("/api/v1/clients")
        .token(&s.recep)
        .json(&json!({ "name": "Bob Vance", "phone": "+1-555-0100" }))
        .send()
        .await
        .expect(StatusCode::CREATED)["id"]
        .as_i64()
        .expect("client id");
    if tier != "Standard" {
        set_tier(app, s, client, tier).await;
    }
    let device = app
        .post("/api/v1/devices")
        .token(&s.recep)
        .json(&json!({
            "client_id": client,
            "device_type": "Laptop",
            "serial_number": format!("SN-{client}"),
            "reported_symptoms": "No power on AC adapter"
        }))
        .send()
        .await
        .expect(StatusCode::CREATED)["id"]
        .as_i64()
        .expect("device id");
    let ticket = format!("FIX-2026-{client:04}");
    let id = app
        .post("/api/v1/orders")
        .token(&s.recep)
        .json(&json!({
            "ticket_number": ticket,
            "client_id": client,
            "device_id": device,
            "assigned_technician_id": s.tech_id
        }))
        .send()
        .await
        .expect(StatusCode::CREATED)["id"]
        .as_i64()
        .expect("order id");
    Order {
        id,
        ticket,
        client,
        device,
        technician: s.tech_id,
    }
}

pub async fn set_tier(app: &TestApp, s: &Staff, client: i64, tier: &str) {
    app.put(&format!("/api/v1/clients/{client}/loyalty-tier"))
        .token(&s.admin)
        .json(&json!({ "loyalty_tier": tier }))
        .send()
        .await
        .expect(StatusCode::OK);
}

/// `PUT /orders/{id}/{endpoint}` with `body` as `token`.
pub async fn put(
    app: &TestApp,
    token: &str,
    order: &Order,
    endpoint: &str,
    body: &Value,
) -> TestResponse {
    app.put(&order.path(endpoint))
        .token(token)
        .json(body)
        .send()
        .await
}

/// The Technician starts diagnosis and saves the contract findings.
pub async fn diagnose(app: &TestApp, s: &Staff, order: &Order) -> Value {
    put(
        app,
        &s.tech,
        order,
        "diagnosis",
        &json!({ "diagnostic_findings": FINDINGS }),
    )
    .await
    .expect(StatusCode::OK)
}

/// The Technician saves an itemized estimate.
pub async fn estimate(app: &TestApp, s: &Staff, order: &Order, parts: i64, labor: i64) -> Value {
    put(
        app,
        &s.tech,
        order,
        "estimate",
        &json!({ "parts_cents": parts, "labor_cents": labor }),
    )
    .await
    .expect(StatusCode::OK)
}

/// The Receptionist records the client's `decision` at [`DECIDED_AT`].
pub async fn decide(app: &TestApp, s: &Staff, order: &Order, decision: &str) -> Value {
    put(
        app,
        &s.recep,
        order,
        "decision",
        &json!({ "decision": decision, "decided_at": DECIDED_AT }),
    )
    .await
    .expect(StatusCode::OK)
}

/// The Technician sends a repair work update.
pub async fn repair(app: &TestApp, s: &Staff, order: &Order, body: &Value) -> Value {
    put(app, &s.tech, order, "repair", body)
        .await
        .expect(StatusCode::OK)
}

/// A `Diagnosing` order with findings, an estimate of `parts`/`labor` and the client's approval.
pub async fn approved_order(app: &TestApp, s: &Staff, tier: &str, parts: i64, labor: i64) -> Order {
    let order = received_order(app, s, tier).await;
    diagnose(app, s, &order).await;
    estimate(app, s, &order, parts, labor).await;
    decide(app, s, &order, "Approved").await;
    order
}

/// An approved $100/$100 order in `InProgress` with nothing incurred yet.
pub async fn in_progress_order(app: &TestApp, s: &Staff, tier: &str) -> Order {
    let order = approved_order(app, s, tier, 10_000, 10_000).await;
    repair(app, s, &order, &json!({ "status": "InProgress" })).await;
    order
}

pub async fn get_order(app: &TestApp, token: &str, order: &Order) -> Value {
    app.get(&format!("/api/v1/orders/{}", order.id))
        .token(token)
        .send()
        .await
        .expect(StatusCode::OK)
}

/// Audit entries of one order, oldest first.
pub async fn order_audit(app: &TestApp, token: &str, order: &Order) -> Vec<Value> {
    app.get(&format!(
        "/api/v1/audit-logs?record_type=RepairOrder&record_id={}",
        order.id
    ))
    .token(token)
    .send()
    .await
    .expect(StatusCode::OK)
    .as_array()
    .expect("JSON array")
    .clone()
}

/// The order and the whole audit log: everything a rejected request could have changed.
pub async fn snapshot(app: &TestApp, token: &str, order: &Order) -> (Value, Value) {
    let log = app
        .get("/api/v1/audit-logs")
        .token(token)
        .send()
        .await
        .expect(StatusCode::OK);
    (get_order(app, token, order).await, log)
}

/// `(field, previous_value, new_value, actor_id)` of each entry, in order.
pub fn changes(entries: &[Value]) -> Vec<(Value, Value, Value, Value)> {
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

/// One expected audit entry.
pub fn change(
    field: &str,
    previous: Value,
    new: Value,
    actor: i64,
) -> (Value, Value, Value, Value) {
    (json!(field), previous, new, json!(actor))
}
