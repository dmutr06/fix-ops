use serde_json::json;

use crate::{ErrorCode, ErrorResponse, MockMode, OrderStatus, RepairOrder, TECH_TOKEN, http};

#[test]
#[ignore = "requires running Prism HTTP mock on 127.0.0.1:4010"]
fn p4_repair_work_and_lifecycle_http_scenarios() {
    // [PREFER-EXAMPLE] Technician records incurred parts/labor, repair check, and moves to Ready (BDD-008, REQ-F-010, REQ-F-011)
    let ready: RepairOrder = http(
        "PUT",
        "/api/v1/orders/100/repair",
        MockMode::PreferExample {
            code: 200,
            example: Some("readyOrderSilver"),
        },
    )
    .bearer(TECH_TOKEN)
    .json(json!({
        "status": "Ready",
        "used_parts_cents": 10000,
        "completed_labor_cents": 10000,
        "repair_checked": true
    }))
    .send_expect(200)
    .json();
    assert_eq!(ready.status, OrderStatus::Ready);
    assert!(ready.repair_checked);
    assert_eq!(ready.labor_discount_cents, 1000);
    assert_eq!(ready.settlement_cents, 19000);

    // [PREFER-EXAMPLE] Entering Ready without repair check rejected with 409 (BDD-009, REQ-F-011, REQ-F-014)
    let err: ErrorResponse = http(
        "PUT",
        "/api/v1/orders/100/repair",
        MockMode::PreferExample {
            code: 409,
            example: Some("missingRepairCheckDenied"),
        },
    )
    .bearer(TECH_TOKEN)
    .json(json!({
        "status": "Ready",
        "repair_checked": false
    }))
    .send_expect(409)
    .json();
    assert_eq!(err.code, ErrorCode::GuardViolation);
    assert_eq!(err.details.unwrap()["guard"], "repair_checked");

    // [PREFER-EXAMPLE] Technician denied cancelling an order with 403 (BDD-019, REQ-F-002, REQ-F-014)
    let err: ErrorResponse = http(
        "POST",
        "/api/v1/orders/100/cancel",
        MockMode::PreferExample {
            code: 403,
            example: Some("technicianCancelDenied"),
        },
    )
    .bearer(TECH_TOKEN)
    .send_expect(403)
    .json();
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert_eq!(err.details.unwrap()["actual_role"], "Technician");
}
