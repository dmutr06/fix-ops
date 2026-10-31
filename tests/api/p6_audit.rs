use serde_json::json;

use crate::{ADMIN_TOKEN, mock};

#[test]
fn p6_administrator_corrections_and_audit_log() {
    let m = mock();

    // Positive: Administrator corrects an order field and queries the audit entry (BDD-016, REQ-F-018, REQ-NF-003)
    let res = m
        .request("PATCH", "/api/v1/orders/100")
        .bearer(ADMIN_TOKEN)
        .json(json!({
            "completed_labor_cents": 8000
        }))
        .send();
    assert_eq!(res.status, 200);
    assert_eq!(res.body["completed_labor_cents"], 8000);

    let res = m
        .request("GET", "/api/v1/audit-logs?record_type=RepairOrder&record_id=100")
        .bearer(ADMIN_TOKEN)
        .send();
    assert_eq!(res.status, 200);
    assert_eq!(res.body[0]["field"], "completed_labor_cents");
    assert_eq!(res.body[0]["previous_value"], "10000");
    assert_eq!(res.body[0]["new_value"], "8000");

    // Negative: Administrator cannot reopen a terminal order (REQ-F-018, ADD-001)
    let res = m
        .request("PATCH", "/api/v1/orders/100")
        .bearer(ADMIN_TOKEN)
        .expect_contract_case(409, "reopenTerminalDenied")
        .json(json!({
            "status": "InProgress"
        }))
        .send();
    assert_eq!(res.status, 409);
    assert_eq!(res.body["code"], "GUARD_VIOLATION");
}
