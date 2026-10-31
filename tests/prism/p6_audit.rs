use serde_json::json;

use crate::{ADMIN, req};

#[test]
fn p6_audit() {
    // BDD-016
    let order = req("PATCH", "/api/v1/orders/100")
        .token(ADMIN)
        .body(json!({ "completed_labor_cents": 8000 }))
        .expect(200);
    assert_eq!(order["completed_labor_cents"], 8000);
    assert_eq!(order["settlement_cents"], 17200);

    // REQ-NF-003
    let log = req(
        "GET",
        "/api/v1/audit-logs?record_type=RepairOrder&record_id=100",
    )
    .token(ADMIN)
    .expect(200);
    assert_eq!(log[0]["field"], "completed_labor_cents");
    assert_eq!(log[0]["previous_value"], "10000");
    assert_eq!(log[0]["new_value"], "8000");

    let err = req("PATCH", "/api/v1/orders/100")
        .token(ADMIN)
        .body(json!({}))
        .invalid()
        .expect(400);
    assert_eq!(err["code"], "VALIDATION_ERROR");

    // REQ-F-018: terminal orders cannot be reopened
    let err = req("PATCH", "/api/v1/orders/100")
        .token(ADMIN)
        .body(json!({ "status": "InProgress" }))
        .prefer(409, "reopenTerminalDenied")
        .expect(409);
    assert_eq!(err["details"]["current_status"], "Completed");
}
