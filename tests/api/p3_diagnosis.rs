use serde_json::json;

use crate::{ADMIN_TOKEN, RECEP_TOKEN, TECH_TOKEN, mock};

#[test]
fn p3_diagnosis_estimate_and_client_approval() {
    let m = mock();

    // Positive: Technician records diagnosis and estimate; Receptionist records approval (BDD-003, REQ-F-006..008)
    let res = m
        .request("PUT", "/api/v1/orders/100/diagnosis")
        .bearer(TECH_TOKEN)
        .json(json!({
            "diagnostic_findings": "Faulty USB-C power controller IC on mainboard"
        }))
        .send();
    assert_eq!(res.status, 200);

    // Positive: Administrator performs a Technician action (BDD-020, REQ-F-002)
    let res = m
        .request("PUT", "/api/v1/orders/100/diagnosis")
        .bearer(ADMIN_TOKEN)
        .json(json!({
            "diagnostic_findings": "Faulty USB-C power controller IC on mainboard"
        }))
        .send();
    assert_eq!(res.status, 200);

    let res = m
        .request("PUT", "/api/v1/orders/100/estimate")
        .bearer(TECH_TOKEN)
        .json(json!({
            "parts_cents": 10000,
            "labor_cents": 10000
        }))
        .send();
    assert_eq!(res.status, 200);

    let res = m
        .request("PUT", "/api/v1/orders/100/decision")
        .bearer(RECEP_TOKEN)
        .json(json!({
            "decision": "Approved",
            "decided_at": "2026-10-21T10:15:00Z"
        }))
        .send();
    assert_eq!(res.status, 200);

    // Negative: entering InProgress without client approval is rejected with 409 (BDD-004, REQ-F-009)
    let res = m
        .request("PUT", "/api/v1/orders/100/repair")
        .bearer(TECH_TOKEN)
        .expect_contract_case(409, "unapprovedRepairDenied")
        .json(json!({ "status": "InProgress" }))
        .send();
    assert_eq!(res.status, 409);
    assert_eq!(res.body["code"], "GUARD_VIOLATION");
    assert_eq!(res.body["details"]["guard"], "client_decision.Approved");
}
