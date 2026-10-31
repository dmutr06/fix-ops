use serde_json::json;

use crate::{
    ADMIN_TOKEN, Client, Device, DeviceType, ErrorCode, ErrorResponse, LoyaltyTier, MockMode,
    OrderStatus, RECEP_TOKEN, RepairOrder, TECH_TOKEN, http,
};

#[test]
#[ignore = "requires running Prism HTTP mock on 127.0.0.1:4010"]
fn p2_intake_and_loyalty_tier_http_scenarios() {
    // [CONTRACT-DEFAULT] Client, Device, and RepairOrder intake (BDD-001, REQ-F-003..005)
    let client: Client = http("POST", "/api/v1/clients", MockMode::ContractDefault)
        .bearer(RECEP_TOKEN)
        .json(json!({
            "name": "Bob Vance",
            "phone": "+1-555-0100",
            "email": "bob@vance.org"
        }))
        .send_expect(201)
        .json();
    assert_eq!(client.loyalty_tier, LoyaltyTier::Standard);

    let device: Device = http("POST", "/api/v1/devices", MockMode::ContractDefault)
        .bearer(RECEP_TOKEN)
        .json(json!({
            "client_id": 10,
            "device_type": "Laptop",
            "serial_number": "SN-THINKPAD-9001",
            "reported_symptoms": "No power on AC adapter"
        }))
        .send_expect(201)
        .json();
    assert_eq!(device.device_type, DeviceType::Laptop);
    assert_eq!(device.serial_number, "SN-THINKPAD-9001");

    let order: RepairOrder = http("POST", "/api/v1/orders", MockMode::ContractDefault)
        .bearer(RECEP_TOKEN)
        .json(json!({
            "ticket_number": "FIX-2026-0001",
            "client_id": 10,
            "device_id": 20,
            "assigned_technician_id": 2
        }))
        .send_expect(201)
        .json();
    assert_eq!(order.status, OrderStatus::Received);
    assert_eq!(order.settlement_cents, 0);

    // [CONTRACT-DEFAULT] Search/list endpoints (REQ-NF-001, ADD-002)
    let clients: Vec<Client> = http("GET", "/api/v1/clients?q=Bob", MockMode::ContractDefault)
        .bearer(TECH_TOKEN)
        .send_expect(200)
        .json();
    assert_eq!(clients[0].name, "Bob Vance");

    // [CONTRACT-DEFAULT] Administrator sets loyalty tier to Gold (BDD-015, REQ-F-017)
    let promoted: Client = http(
        "PUT",
        "/api/v1/clients/10/loyalty-tier",
        MockMode::ContractDefault,
    )
    .bearer(ADMIN_TOKEN)
    .json(json!({ "loyalty_tier": "Gold" }))
    .send_expect(200)
    .json();
    assert_eq!(promoted.loyalty_tier, LoyaltyTier::Gold);

    // [AUTO-VALIDATION] Empty serial_number automatically rejected by Prism body schema check (BDD-002, REQ-F-004)
    let err: ErrorResponse = http("POST", "/api/v1/devices", MockMode::AutoValidation)
        .bearer(RECEP_TOKEN)
        .json(json!({
            "client_id": 10,
            "device_type": "Laptop",
            "serial_number": "",
            "reported_symptoms": "Broken hinge"
        }))
        .send_expect(400)
        .json();
    assert_eq!(err.code, ErrorCode::ValidationError);
    assert_eq!(err.details.unwrap()["field"], "serial_number");

    // [AUTO-VALIDATION] Invalid path parameter (orderId < 1) automatically rejected with 400
    let err: ErrorResponse = http("GET", "/api/v1/orders/0", MockMode::AutoValidation)
        .bearer(RECEP_TOKEN)
        .send_expect(400)
        .json();
    assert_eq!(err.code, ErrorCode::ValidationError);

    // [PREFER-EXAMPLE] Technician denied changing loyalty tier (BDD-011, REQ-F-002, REQ-F-017)
    let err: ErrorResponse = http(
        "PUT",
        "/api/v1/clients/10/loyalty-tier",
        MockMode::PreferExample {
            code: 403,
            example: Some("technicianDenied"),
        },
    )
    .bearer(TECH_TOKEN)
    .json(json!({ "loyalty_tier": "Gold" }))
    .send_expect(403)
    .json();
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert_eq!(err.details.unwrap()["actual_role"], "Technician");

    // [PREFER-EXAMPLE] Unknown order ID returns 404 Not Found
    let err: ErrorResponse = http(
        "GET",
        "/api/v1/orders/9999",
        MockMode::PreferExample {
            code: 404,
            example: None,
        },
    )
    .bearer(RECEP_TOKEN)
    .send_expect(404)
    .json();
    assert_eq!(err.code, ErrorCode::NotFound);

    // [PREFER-EXAMPLE] Duplicate ticket number returns 409 Conflict (BDD-002, REQ-F-005)
    let err: ErrorResponse = http(
        "POST",
        "/api/v1/orders",
        MockMode::PreferExample {
            code: 409,
            example: Some("duplicateTicket"),
        },
    )
    .bearer(RECEP_TOKEN)
    .json(json!({
        "ticket_number": "FIX-2026-0001",
        "client_id": 10,
        "device_id": 20,
        "assigned_technician_id": 2
    }))
    .send_expect(409)
    .json();
    assert_eq!(err.code, ErrorCode::Conflict);
    assert_eq!(err.details.unwrap()["ticket_number"], "FIX-2026-0001");
}
