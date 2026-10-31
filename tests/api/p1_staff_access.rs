use serde_json::json;

use crate::{ADMIN_TOKEN, RECEP_TOKEN, mock};

#[test]
fn p1_staff_sign_in_and_account_management() {
    let m = mock();

    // Positive: active staff sign-in (REQ-F-001)
    let res = m
        .request("POST", "/api/v1/sessions")
        .json(json!({
            "username": "anna.recep",
            "password": "correct-horse-battery-staple"
        }))
        .send();
    assert_eq!(res.status, 201);
    assert_eq!(res.body["token"], RECEP_TOKEN);
    assert_eq!(res.body["staff"]["role"], "Receptionist");

    // Positive: Administrator creates and disables a staff account (REQ-F-019)
    let res = m
        .request("POST", "/api/v1/staff")
        .bearer(ADMIN_TOKEN)
        .json(json!({
            "username": "max.tech",
            "password": "initial-password-123",
            "role": "Technician"
        }))
        .send();
    assert_eq!(res.status, 201);
    assert_eq!(res.body["role"], "Technician");

    let res = m
        .request("PATCH", "/api/v1/staff/2")
        .bearer(ADMIN_TOKEN)
        .json(json!({ "enabled": false }))
        .send();
    assert_eq!(res.status, 200);

    // Negative: unauthenticated request is rejected with 401 (BDD-017, REQ-F-001)
    let res = m.request("GET", "/api/v1/orders/100").send();
    assert_eq!(res.status, 401);
    assert_eq!(res.body["code"], "UNAUTHENTICATED");

    // Negative: disabled account cannot sign in (REQ-F-019)
    let res = m
        .request("POST", "/api/v1/sessions")
        .json(json!({
            "username": "disabled.user",
            "password": "password123"
        }))
        .send();
    assert_eq!(res.status, 403);
    assert_eq!(res.body["code"], "FORBIDDEN");

    // Negative: non-Administrator cannot manage staff accounts (REQ-F-002, REQ-F-019)
    let res = m.request("GET", "/api/v1/staff").bearer(RECEP_TOKEN).send();
    assert_eq!(res.status, 403);
    assert_eq!(res.body["code"], "FORBIDDEN");
}
