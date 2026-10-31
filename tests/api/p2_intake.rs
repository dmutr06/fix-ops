use serde_json::json;

use crate::{ADMIN_TOKEN, RECEP_TOKEN, TECH_TOKEN, mock};

#[test]
fn p2_client_device_order_intake_and_loyalty_tier() {
    let m = mock();

    // Positive: Client, Device, and RepairOrder intake in Received state (BDD-001, REQ-F-003..005)
    let res = m
        .request("POST", "/api/v1/clients")
        .bearer(RECEP_TOKEN)
        .json(json!({
            "name": "Bob Vance",
            "phone": "+1-555-0100",
            "email": "bob@vance.org"
        }))
        .send();
    assert_eq!(res.status, 201);
    assert_eq!(res.body["loyalty_tier"], "Standard");

    let res = m
        .request("POST", "/api/v1/devices")
        .bearer(RECEP_TOKEN)
        .json(json!({
            "client_id": 10,
            "device_type": "Laptop",
            "serial_number": "SN-THINKPAD-9001",
            "reported_symptoms": "No power on AC adapter"
        }))
        .send();
    assert_eq!(res.status, 201);
    assert_eq!(res.body["serial_number"], "SN-THINKPAD-9001");

    let res = m
        .request("POST", "/api/v1/orders")
        .bearer(RECEP_TOKEN)
        .json(json!({
            "ticket_number": "FIX-2026-0001",
            "client_id": 10,
            "device_id": 20,
            "assigned_technician_id": 2
        }))
        .send();
    assert_eq!(res.status, 201);
    assert_eq!(res.body["status"], "Received");
    assert_eq!(res.body["settlement_cents"], 0);

    // Positive: Staff search/list clients, devices, and orders (REQ-F-003..005, REQ-NF-001)
    let res = m
        .request("GET", "/api/v1/clients?q=Bob")
        .bearer(TECH_TOKEN)
        .send();
    assert_eq!(res.status, 200);
    assert_eq!(res.body[0]["name"], "Bob Vance");

    let res = m
        .request("GET", "/api/v1/devices?client_id=10")
        .bearer(TECH_TOKEN)
        .send();
    assert_eq!(res.status, 200);
    assert_eq!(res.body[0]["serial_number"], "SN-THINKPAD-9001");

    let res = m
        .request("GET", "/api/v1/orders?status=Received")
        .bearer(TECH_TOKEN)
        .send();
    assert_eq!(res.status, 200);
    assert_eq!(res.body[0]["ticket_number"], "FIX-2026-0001");

    // Positive: Administrator sets loyalty tier to Gold (BDD-015, REQ-F-017)
    let res = m
        .request("PUT", "/api/v1/clients/10/loyalty-tier")
        .bearer(ADMIN_TOKEN)
        .json(json!({ "loyalty_tier": "Gold" }))
        .send();
    assert_eq!(res.status, 200);
    assert_eq!(res.body["loyalty_tier"], "Gold");

    // Negative: Technician denied changing loyalty tier (BDD-011, REQ-F-002, REQ-F-017)
    let res = m
        .request("PUT", "/api/v1/clients/10/loyalty-tier")
        .bearer(TECH_TOKEN)
        .json(json!({ "loyalty_tier": "Gold" }))
        .send();
    assert_eq!(res.status, 403);
    assert_eq!(res.body["code"], "FORBIDDEN");

    // Negative: missing device serial_number rejected with 400 (BDD-002, REQ-F-004)
    let res = m
        .request("POST", "/api/v1/devices")
        .bearer(RECEP_TOKEN)
        .json(json!({
            "client_id": 10,
            "device_type": "Laptop",
            "serial_number": "",
            "reported_symptoms": "Broken hinge"
        }))
        .send();
    assert_eq!(res.status, 400);
    assert_eq!(res.body["code"], "VALIDATION_ERROR");
    assert_eq!(res.body["details"]["field"], "serial_number");

    // Negative: duplicate ticket number rejected with 409 (BDD-002, REQ-F-005)
    let res = m
        .request("POST", "/api/v1/orders")
        .bearer(RECEP_TOKEN)
        .expect_contract_case(409, "duplicateTicket")
        .json(json!({
            "ticket_number": "FIX-2026-0001",
            "client_id": 10,
            "device_id": 20,
            "assigned_technician_id": 2
        }))
        .send();
    assert_eq!(res.status, 409);
    assert_eq!(res.body["code"], "CONFLICT");
}
