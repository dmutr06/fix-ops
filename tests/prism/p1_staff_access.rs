use serde_json::json;

use crate::{
    ADMIN_TOKEN, ErrorCode, ErrorResponse, MockMode, RECEP_TOKEN, Role, SessionResponse,
    StaffAccount, http,
};

#[test]
#[ignore = "requires running Prism HTTP mock on 127.0.0.1:4010"]
fn p1_staff_access_http_scenarios() {
    // [CONTRACT-DEFAULT] Active staff sign-in (REQ-F-001)
    let session: SessionResponse = http("POST", "/api/v1/sessions", MockMode::ContractDefault)
        .json(json!({
            "username": "anna.recep",
            "password": "correct-horse-battery-staple"
        }))
        .send_expect(201)
        .json();
    assert_eq!(session.token, RECEP_TOKEN);
    assert_eq!(session.staff.role, Role::Receptionist);
    assert!(session.staff.enabled);

    // [CONTRACT-DEFAULT] Administrator creates and disables a staff account (REQ-F-019)
    let created: StaffAccount = http("POST", "/api/v1/staff", MockMode::ContractDefault)
        .bearer(ADMIN_TOKEN)
        .json(json!({
            "username": "max.tech",
            "password": "initial-password-123",
            "role": "Technician"
        }))
        .send_expect(201)
        .json();
    assert_eq!(created.role, Role::Technician);
    assert!(created.enabled);

    let updated: StaffAccount = http("PATCH", "/api/v1/staff/2", MockMode::ContractDefault)
        .bearer(ADMIN_TOKEN)
        .json(json!({ "enabled": false }))
        .send_expect(200)
        .json();
    assert!(!updated.enabled);

    let staff_list: Vec<StaffAccount> = http("GET", "/api/v1/staff", MockMode::ContractDefault)
        .bearer(ADMIN_TOKEN)
        .send_expect(200)
        .json();
    assert!(!staff_list.is_empty());

    // [AUTO-VALIDATION] Unauthenticated request automatically rejected by Prism security check (BDD-017, REQ-F-001)
    let err: ErrorResponse = http("GET", "/api/v1/orders/100", MockMode::AutoValidation)
        .send_expect(401)
        .json();
    assert_eq!(err.code, ErrorCode::Unauthenticated);

    // [PREFER-EXAMPLE] Disabled account sign-in rejected with 403 (REQ-F-019)
    let err: ErrorResponse = http(
        "POST",
        "/api/v1/sessions",
        MockMode::PreferExample {
            code: 403,
            example: Some("disabledAccount"),
        },
    )
    .json(json!({
        "username": "disabled.user",
        "password": "password123"
    }))
    .send_expect(403)
    .json();
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert_eq!(err.details.unwrap()["username"], "disabled.user");

    // [PREFER-EXAMPLE] Non-Administrator denied staff directory access (REQ-F-002, ADD-002)
    let err: ErrorResponse = http(
        "GET",
        "/api/v1/staff",
        MockMode::PreferExample {
            code: 403,
            example: None,
        },
    )
    .bearer(RECEP_TOKEN)
    .send_expect(403)
    .json();
    assert_eq!(err.code, ErrorCode::Forbidden);
}
