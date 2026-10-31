use serde_json::json;

use crate::{TECH, req};

#[test]
#[ignore = "needs Prism on 127.0.0.1:4010"]
fn p4_repair() {
    // BDD-008: Silver, $100 parts + $100 labor = $190
    let order = req("PUT", "/api/v1/orders/100/repair")
        .token(TECH)
        .body(json!({ "status": "Ready", "used_parts_cents": 10000, "completed_labor_cents": 10000, "repair_checked": true }))
        .prefer(200, "readyOrderSilver")
        .expect(200);
    assert_eq!(order["status"], "Ready");
    assert_eq!(order["labor_discount_cents"], 1000);
    assert_eq!(order["settlement_cents"], 19000);

    // BDD-014: Silver discount on $0.05 labor rounds up to $0.01
    let order = req("PUT", "/api/v1/orders/104/repair")
        .token(TECH)
        .body(json!({ "status": "Ready", "used_parts_cents": 0, "completed_labor_cents": 5, "repair_checked": true }))
        .prefer(200, "readyOrderHalfCentSilver")
        .expect(200);
    assert_eq!(order["labor_discount_cents"], 1);
    assert_eq!(order["settlement_cents"], 4);

    // BDD-009
    let err = req("PUT", "/api/v1/orders/100/repair")
        .token(TECH)
        .body(json!({ "status": "Ready", "repair_checked": false }))
        .prefer(409, "missingRepairCheckDenied")
        .expect(409);
    assert_eq!(err["details"]["guard"], "repair_checked");

    // BDD-019
    let err = req("POST", "/api/v1/orders/100/cancel")
        .token(TECH)
        .prefer(403, "technicianCancelDenied")
        .expect(403);
    assert_eq!(err["code"], "FORBIDDEN");
}
