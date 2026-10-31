use serde_json::json;

use crate::{
    ADMIN_TOKEN, AuditEntry, AuditRecordType, ErrorCode, ErrorResponse, MockMode, RepairOrder, http,
};

#[test]
#[ignore = "requires running Prism HTTP mock on 127.0.0.1:4010"]
fn p6_admin_corrections_and_audit_http_scenarios() {
    // [CONTRACT-DEFAULT] Administrator corrects completed labor charge (BDD-016, REQ-F-018)
    let corrected: RepairOrder = http("PATCH", "/api/v1/orders/100", MockMode::ContractDefault)
        .bearer(ADMIN_TOKEN)
        .json(json!({ "completed_labor_cents": 8000 }))
        .send_expect(200)
        .json();
    assert_eq!(corrected.completed_labor_cents, 8000);
    assert_eq!(corrected.settlement_cents, 17200);

    // [CONTRACT-DEFAULT] Audit log query returns actor, time, record, field, previous and new values (REQ-NF-003)
    let entries: Vec<AuditEntry> = http(
        "GET",
        "/api/v1/audit-logs?record_type=RepairOrder&record_id=100",
        MockMode::ContractDefault,
    )
    .bearer(ADMIN_TOKEN)
    .send_expect(200)
    .json();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].record_type, AuditRecordType::RepairOrder);
    assert_eq!(entries[0].field, "completed_labor_cents");
    assert_eq!(entries[0].previous_value.as_deref(), Some("10000"));
    assert_eq!(entries[0].new_value.as_deref(), Some("8000"));

    // [AUTO-VALIDATION] Empty PATCH body violates minProperties: 1 and is automatically rejected with 400
    let err: ErrorResponse = http("PATCH", "/api/v1/orders/100", MockMode::AutoValidation)
        .bearer(ADMIN_TOKEN)
        .json(json!({}))
        .send_expect(400)
        .json();
    assert_eq!(err.code, ErrorCode::ValidationError);

    // [PREFER-EXAMPLE] Administrator cannot reopen a terminal order (REQ-F-018, ADD-001)
    let err: ErrorResponse = http(
        "PATCH",
        "/api/v1/orders/100",
        MockMode::PreferExample {
            code: 409,
            example: Some("reopenTerminalDenied"),
        },
    )
    .bearer(ADMIN_TOKEN)
    .json(json!({ "status": "InProgress" }))
    .send_expect(409)
    .json();
    assert_eq!(err.code, ErrorCode::GuardViolation);
    assert_eq!(err.details.unwrap()["current_status"], "Completed");
}
