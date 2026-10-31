use serde_json::json;

use crate::{ADMIN_TOKEN, RECEP_TOKEN, mock};

#[test]
fn p5_settlement_cancellation_payment_and_pickup() {
    let m = mock();

    // Positive: Receptionist records payment, pickup, and completes the Ready order (REQ-F-012, REQ-F-013, REQ-F-016)
    let res = m
        .request("PUT", "/api/v1/orders/100/payment")
        .bearer(RECEP_TOKEN)
        .json(json!({
            "amount_cents": 19000,
            "paid_at": "2026-10-23T15:00:00Z"
        }))
        .send();
    assert_eq!(res.status, 200);

    let res = m
        .request("PUT", "/api/v1/orders/100/pickup")
        .bearer(RECEP_TOKEN)
        .json(json!({
            "picked_up_at": "2026-10-23T15:05:00Z"
        }))
        .send();
    assert_eq!(res.status, 200);

    let res = m
        .request("POST", "/api/v1/orders/100/complete")
        .bearer(RECEP_TOKEN)
        .send();
    assert_eq!(res.status, 200);
    assert_eq!(res.body["status"], "Completed");

    // Positive: stage-based cancellation charges ($0 before findings, $30 after findings, incurred costs in progress) (BDD-005..007, REQ-F-015)
    let res = m
        .request("POST", "/api/v1/orders/101/cancel")
        .bearer(RECEP_TOKEN)
        .expect_contract_case(200, "cancelledBeforeFindings")
        .send();
    assert_eq!(res.status, 200);
    assert_eq!(res.body["settlement_cents"], 0);

    let res = m
        .request("POST", "/api/v1/orders/102/cancel")
        .bearer(RECEP_TOKEN)
        .expect_contract_case(200, "cancelledAfterFindings")
        .send();
    assert_eq!(res.status, 200);
    assert_eq!(res.body["settlement_cents"], 3000);

    let res = m
        .request("POST", "/api/v1/orders/103/cancel")
        .bearer(ADMIN_TOKEN)
        .expect_contract_case(200, "cancelledInProgressGold")
        .send();
    assert_eq!(res.status, 200);
    assert_eq!(res.body["settlement_cents"], 14000);

    // Negative: missing pickup prevents order completion (BDD-010, REQ-F-013)
    let res = m
        .request("POST", "/api/v1/orders/100/complete")
        .bearer(RECEP_TOKEN)
        .expect_contract_case(409, "missingPickupDenied")
        .send();
    assert_eq!(res.status, 409);
    assert_eq!(res.body["code"], "GUARD_VIOLATION");
    assert_eq!(res.body["details"]["guard"], "pickup");

    // Negative: short payment prevents order completion (BDD-012, REQ-F-012, REQ-F-013)
    let res = m
        .request("POST", "/api/v1/orders/100/complete")
        .bearer(RECEP_TOKEN)
        .expect_contract_case(409, "shortPaymentDenied")
        .send();
    assert_eq!(res.status, 409);
    assert_eq!(res.body["code"], "GUARD_VIOLATION");
    assert_eq!(res.body["details"]["paid_cents"], 10000);
}
