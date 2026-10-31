use serde_json::json;

use crate::{ADMIN, RECEP, req};

#[test]
#[ignore = "needs Prism on 127.0.0.1:4010"]
fn p1_staff_access() {
    // REQ-F-001
    let session = req("POST", "/api/v1/sessions")
        .body(json!({ "username": "anna.recep", "password": "correct-horse-battery-staple" }))
        .expect(201);
    assert_eq!(session["token"], RECEP);
    assert_eq!(session["staff"]["role"], "Receptionist");

    // REQ-F-019
    let created = req("POST", "/api/v1/staff")
        .token(ADMIN)
        .body(json!({ "username": "max.tech", "password": "initial-password-123", "role": "Technician" }))
        .expect(201);
    assert_eq!(created["role"], "Technician");

    let disabled = req("PATCH", "/api/v1/staff/2")
        .token(ADMIN)
        .body(json!({ "enabled": false }))
        .expect(200);
    assert_eq!(disabled["enabled"], false);

    let staff = req("GET", "/api/v1/staff").token(ADMIN).expect(200);
    assert!(!staff.as_array().unwrap().is_empty());

    // BDD-017
    let err = req("GET", "/api/v1/orders/100").invalid().expect(401);
    assert_eq!(err["code"], "UNAUTHENTICATED");

    // REQ-F-019: disabled account cannot sign in
    let err = req("POST", "/api/v1/sessions")
        .body(json!({ "username": "disabled.user", "password": "password123" }))
        .prefer(403, "disabledAccount")
        .expect(403);
    assert_eq!(err["code"], "FORBIDDEN");

    // ADD-002: only Administrator sees staff accounts
    let err = req("GET", "/api/v1/staff")
        .token(RECEP)
        .prefer(403, "roleForbidden")
        .expect(403);
    assert_eq!(err["code"], "FORBIDDEN");
}
