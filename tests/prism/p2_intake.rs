use serde_json::json;

use crate::{ADMIN, RECEP, TECH, req};

#[test]
fn p2_intake() {
    // BDD-001
    let client = req("POST", "/api/v1/clients")
        .token(RECEP)
        .body(json!({ "name": "Bob Vance", "phone": "+1-555-0100", "email": "bob@vance.org" }))
        .expect(201);
    assert_eq!(client["loyalty_tier"], "Standard");

    let device = req("POST", "/api/v1/devices")
        .token(RECEP)
        .body(json!({
            "client_id": 10,
            "device_type": "Laptop",
            "serial_number": "SN-THINKPAD-9001",
            "reported_symptoms": "No power on AC adapter"
        }))
        .expect(201);
    assert_eq!(device["serial_number"], "SN-THINKPAD-9001");

    let order = req("POST", "/api/v1/orders")
        .token(RECEP)
        .body(json!({ "ticket_number": "FIX-2026-0001", "client_id": 10, "device_id": 20, "assigned_technician_id": 2 }))
        .expect(201);
    assert_eq!(order["status"], "Received");

    // ADD-002: any staff member can search
    let clients = req("GET", "/api/v1/clients?q=Bob").token(TECH).expect(200);
    assert_eq!(clients[0]["name"], "Bob Vance");

    // BDD-015
    let client = req("PUT", "/api/v1/clients/10/loyalty-tier")
        .token(ADMIN)
        .body(json!({ "loyalty_tier": "Gold" }))
        .expect(200);
    assert_eq!(client["loyalty_tier"], "Gold");

    // BDD-002: empty serial number
    let err = req("POST", "/api/v1/devices")
        .token(RECEP)
        .body(json!({
            "client_id": 10,
            "device_type": "Laptop",
            "serial_number": "",
            "reported_symptoms": "Broken hinge"
        }))
        .invalid()
        .expect(400);
    assert_eq!(err["code"], "VALIDATION_ERROR");

    let err = req("GET", "/api/v1/orders/0")
        .token(RECEP)
        .invalid()
        .expect(400);
    assert_eq!(err["code"], "VALIDATION_ERROR");

    // BDD-011
    let err = req("PUT", "/api/v1/clients/10/loyalty-tier")
        .token(TECH)
        .body(json!({ "loyalty_tier": "Gold" }))
        .prefer(403, "technicianDenied")
        .expect(403);
    assert_eq!(err["details"]["actual_role"], "Technician");

    let err = req("GET", "/api/v1/orders/9999")
        .token(RECEP)
        .prefer(404, "notFound")
        .expect(404);
    assert_eq!(err["code"], "NOT_FOUND");

    // BDD-002: duplicate ticket number
    let err = req("POST", "/api/v1/orders")
        .token(RECEP)
        .body(json!({ "ticket_number": "FIX-2026-0001", "client_id": 10, "device_id": 20, "assigned_technician_id": 2 }))
        .prefer(409, "duplicateTicket")
        .expect(409);
    assert_eq!(err["code"], "CONFLICT");
}
