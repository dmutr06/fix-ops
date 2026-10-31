use serde_json::json;

use crate::{ADMIN, RECEP, TECH, req};

#[test]
fn p3_diagnosis() {
    let findings =
        json!({ "diagnostic_findings": "Faulty USB-C power controller IC on mainboard" });

    // REQ-F-006
    let order = req("PUT", "/api/v1/orders/100/diagnosis")
        .token(TECH)
        .body(findings.clone())
        .expect(200);
    assert_eq!(order["status"], "Diagnosing");

    // BDD-020: Administrator can do Technician work
    let order = req("PUT", "/api/v1/orders/100/diagnosis")
        .token(ADMIN)
        .body(findings)
        .expect(200);
    assert_eq!(order["status"], "Diagnosing");

    // REQ-F-007
    let order = req("PUT", "/api/v1/orders/100/estimate")
        .token(TECH)
        .body(json!({ "parts_cents": 10000, "labor_cents": 10000 }))
        .expect(200);
    assert_eq!(
        order["estimate"],
        json!({ "parts_cents": 10000, "labor_cents": 10000 })
    );

    // BDD-003
    let order = req("PUT", "/api/v1/orders/100/decision")
        .token(RECEP)
        .body(json!({ "decision": "Approved", "decided_at": "2026-10-21T10:15:00Z" }))
        .expect(200);
    assert_eq!(order["client_decision"]["decision"], "Approved");

    // AC-F-008: decision without a time
    let err = req("PUT", "/api/v1/orders/100/decision")
        .token(RECEP)
        .body(json!({ "decision": "Approved" }))
        .invalid()
        .expect(400);
    assert_eq!(err["code"], "VALIDATION_ERROR");

    // BDD-004
    let err = req("PUT", "/api/v1/orders/100/repair")
        .token(TECH)
        .body(json!({ "status": "InProgress" }))
        .prefer(409, "unapprovedRepairDenied")
        .expect(409);
    assert_eq!(err["code"], "GUARD_VIOLATION");
}
