use serde_json::json;

use crate::{TECH_TOKEN, mock};

#[test]
fn p4_repair_work_and_lifecycle_guards() {
    let m = mock();

    // Positive: Technician records used parts, completed labor, repair check, and moves to Ready (BDD-008, REQ-F-010, REQ-F-011)
    let res = m
        .request("PUT", "/api/v1/orders/100/repair")
        .bearer(TECH_TOKEN)
        .expect_contract_case(200, "readyOrderSilver")
        .json(json!({
            "status": "Ready",
            "used_parts_cents": 10000,
            "completed_labor_cents": 10000,
            "repair_checked": true
        }))
        .send();
    assert_eq!(res.status, 200);
    assert_eq!(res.body["status"], "Ready");
    assert_eq!(res.body["labor_discount_cents"], 1000);
    assert_eq!(res.body["settlement_cents"], 19000);

    // Negative: entering Ready without repair check is rejected with 409 (BDD-009, REQ-F-011, REQ-F-014)
    let res = m
        .request("PUT", "/api/v1/orders/100/repair")
        .bearer(TECH_TOKEN)
        .expect_contract_case(409, "missingRepairCheckDenied")
        .json(json!({
            "status": "Ready",
            "repair_checked": false
        }))
        .send();
    assert_eq!(res.status, 409);
    assert_eq!(res.body["code"], "GUARD_VIOLATION");
    assert_eq!(res.body["details"]["guard"], "repair_checked");

    // Negative: Technician cannot cancel an order (BDD-019, REQ-F-002, REQ-F-014)
    let res = m
        .request("POST", "/api/v1/orders/100/cancel")
        .bearer(TECH_TOKEN)
        .send();
    assert_eq!(res.status, 403);
    assert_eq!(res.body["code"], "FORBIDDEN");
}
