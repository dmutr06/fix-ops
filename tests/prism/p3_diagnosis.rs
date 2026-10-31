use serde_json::json;

use crate::{
    DecisionType, ErrorCode, ErrorResponse, Estimate, MockMode, OrderStatus, RECEP_TOKEN,
    RepairOrder, TECH_TOKEN, http,
};

#[test]
#[ignore = "requires running Prism HTTP mock on 127.0.0.1:4010"]
fn p3_diagnosis_estimate_and_approval_http_scenarios() {
    // [CONTRACT-DEFAULT] Technician moves order to Diagnosing and saves findings (REQ-F-006)
    let diagnosed: RepairOrder = http(
        "PUT",
        "/api/v1/orders/100/diagnosis",
        MockMode::ContractDefault,
    )
    .bearer(TECH_TOKEN)
    .json(json!({
        "diagnostic_findings": "Faulty USB-C power controller IC on mainboard"
    }))
    .send_expect(200)
    .json();
    assert_eq!(diagnosed.status, OrderStatus::Diagnosing);
    assert!(diagnosed.diagnostic_findings.is_some());

    // [CONTRACT-DEFAULT] Technician prepares itemized estimate in cents (REQ-F-007)
    let estimated: RepairOrder = http(
        "PUT",
        "/api/v1/orders/100/estimate",
        MockMode::ContractDefault,
    )
    .bearer(TECH_TOKEN)
    .json(json!({
        "parts_cents": 10000,
        "labor_cents": 10000
    }))
    .send_expect(200)
    .json();
    assert_eq!(
        estimated.estimate,
        Some(Estimate {
            parts_cents: 10000,
            labor_cents: 10000
        })
    );

    // [CONTRACT-DEFAULT] Receptionist records client approval and timestamp (BDD-003, REQ-F-008)
    let approved: RepairOrder = http(
        "PUT",
        "/api/v1/orders/100/decision",
        MockMode::ContractDefault,
    )
    .bearer(RECEP_TOKEN)
    .json(json!({
        "decision": "Approved",
        "decided_at": "2026-10-21T10:15:00Z"
    }))
    .send_expect(200)
    .json();
    assert_eq!(
        approved.client_decision.unwrap().decision,
        DecisionType::Approved
    );

    // [AUTO-VALIDATION] Decision missing required timestamp rejected automatically with 400 (AC-F-008)
    let err: ErrorResponse = http(
        "PUT",
        "/api/v1/orders/100/decision",
        MockMode::AutoValidation,
    )
    .bearer(RECEP_TOKEN)
    .json(json!({ "decision": "Approved" }))
    .send_expect(400)
    .json();
    assert_eq!(err.code, ErrorCode::ValidationError);

    // [PREFER-EXAMPLE] Entering InProgress without recorded client approval rejected with 409 (BDD-004, REQ-F-009)
    let err: ErrorResponse = http(
        "PUT",
        "/api/v1/orders/100/repair",
        MockMode::PreferExample {
            code: 409,
            example: Some("unapprovedRepairDenied"),
        },
    )
    .bearer(TECH_TOKEN)
    .json(json!({ "status": "InProgress" }))
    .send_expect(409)
    .json();
    assert_eq!(err.code, ErrorCode::GuardViolation);
    assert_eq!(err.details.unwrap()["guard"], "client_decision.Approved");
}
