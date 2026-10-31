use serde_json::json;

use crate::{ADMIN, RECEP, req};

#[test]
fn p5_settlement() {
    // BDD-010: payment and pickup, then complete
    let order = req("PUT", "/api/v1/orders/100/payment")
        .token(RECEP)
        .body(json!({ "amount_cents": 19000, "paid_at": "2026-10-23T15:00:00Z" }))
        .expect(200);
    assert_eq!(order["payment"]["amount_cents"], 19000);

    let order = req("PUT", "/api/v1/orders/100/pickup")
        .token(RECEP)
        .body(json!({ "picked_up_at": "2026-10-23T15:05:00Z" }))
        .expect(200);
    assert!(order["pickup"].is_object());

    let order = req("POST", "/api/v1/orders/100/complete")
        .token(RECEP)
        .expect(200);
    assert_eq!(order["status"], "Completed");
    assert_eq!(order["settlement_cents"], 19000);

    // BDD-005, BDD-013, BDD-018: cancel before findings costs $0
    let order = req("POST", "/api/v1/orders/101/cancel")
        .token(RECEP)
        .prefer(200, "cancelledBeforeFindings")
        .expect(200);
    assert_eq!(order["status"], "Cancelled");
    assert_eq!(order["settlement_cents"], 0);

    // BDD-006: cancel after findings costs the $30 fee
    let order = req("POST", "/api/v1/orders/102/cancel")
        .token(RECEP)
        .prefer(200, "cancelledAfterFindings")
        .expect(200);
    assert_eq!(order["settlement_cents"], 3000);

    // BDD-007, BDD-018: Administrator cancels during repair, Gold, $140
    let order = req("POST", "/api/v1/orders/103/cancel")
        .token(ADMIN)
        .prefer(200, "cancelledInProgressGold")
        .expect(200);
    assert_eq!(order["settlement_cents"], 14000);

    // BDD-010: no pickup
    let err = req("POST", "/api/v1/orders/100/complete")
        .token(RECEP)
        .prefer(409, "missingPickupDenied")
        .expect(409);
    assert_eq!(err["details"]["guard"], "pickup");

    // BDD-012: short payment
    let err = req("POST", "/api/v1/orders/100/complete")
        .token(RECEP)
        .prefer(409, "shortPaymentDenied")
        .expect(409);
    assert_eq!(err["details"]["paid_cents"], 10000);
}
