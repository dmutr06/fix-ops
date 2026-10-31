use serde_json::json;

use crate::{
    ADMIN_TOKEN, ErrorCode, ErrorResponse, MockMode, OrderStatus, RECEP_TOKEN, RepairOrder, http,
};

#[test]
#[ignore = "requires running Prism HTTP mock on 127.0.0.1:4010"]
fn p5_settlement_payment_pickup_and_cancellation_http_scenarios() {
    // [CONTRACT-DEFAULT] Record full payment, pickup, and complete order (REQ-F-012, REQ-F-013, REQ-F-016)
    let paid: RepairOrder = http(
        "PUT",
        "/api/v1/orders/100/payment",
        MockMode::ContractDefault,
    )
    .bearer(RECEP_TOKEN)
    .json(json!({
        "amount_cents": 19000,
        "paid_at": "2026-10-23T15:00:00Z"
    }))
    .send_expect(200)
    .json();
    assert_eq!(paid.payment.unwrap().amount_cents, 19000);

    let picked_up: RepairOrder = http(
        "PUT",
        "/api/v1/orders/100/pickup",
        MockMode::ContractDefault,
    )
    .bearer(RECEP_TOKEN)
    .json(json!({
        "picked_up_at": "2026-10-23T15:05:00Z"
    }))
    .send_expect(200)
    .json();
    assert!(picked_up.pickup.is_some());

    let completed: RepairOrder = http(
        "POST",
        "/api/v1/orders/100/complete",
        MockMode::ContractDefault,
    )
    .bearer(RECEP_TOKEN)
    .send_expect(200)
    .json();
    assert_eq!(completed.status, OrderStatus::Completed);
    assert_eq!(completed.settlement_cents, 19000);

    // [PREFER-EXAMPLE] Stage-based cancellation charges ($0, $30, $140) (BDD-005..007, REQ-F-015)
    let c0: RepairOrder = http(
        "POST",
        "/api/v1/orders/101/cancel",
        MockMode::PreferExample {
            code: 200,
            example: Some("cancelledBeforeFindings"),
        },
    )
    .bearer(RECEP_TOKEN)
    .send_expect(200)
    .json();
    assert_eq!(c0.status, OrderStatus::Cancelled);
    assert_eq!(c0.settlement_cents, 0);

    let c30: RepairOrder = http(
        "POST",
        "/api/v1/orders/102/cancel",
        MockMode::PreferExample {
            code: 200,
            example: Some("cancelledAfterFindings"),
        },
    )
    .bearer(RECEP_TOKEN)
    .send_expect(200)
    .json();
    assert_eq!(c30.status, OrderStatus::Cancelled);
    assert_eq!(c30.settlement_cents, 3000);

    let c140: RepairOrder = http(
        "POST",
        "/api/v1/orders/103/cancel",
        MockMode::PreferExample {
            code: 200,
            example: Some("cancelledInProgressGold"),
        },
    )
    .bearer(ADMIN_TOKEN)
    .send_expect(200)
    .json();
    assert_eq!(c140.status, OrderStatus::Cancelled);
    assert_eq!(c140.settlement_cents, 14000);

    // [PREFER-EXAMPLE] Missing pickup prevents completion with 409 (BDD-010, REQ-F-013)
    let err: ErrorResponse = http(
        "POST",
        "/api/v1/orders/100/complete",
        MockMode::PreferExample {
            code: 409,
            example: Some("missingPickupDenied"),
        },
    )
    .bearer(RECEP_TOKEN)
    .send_expect(409)
    .json();
    assert_eq!(err.code, ErrorCode::GuardViolation);
    assert_eq!(err.details.unwrap()["guard"], "pickup");

    // [PREFER-EXAMPLE] Short payment prevents completion with 409 (BDD-012, REQ-F-012, REQ-F-013)
    let err: ErrorResponse = http(
        "POST",
        "/api/v1/orders/100/complete",
        MockMode::PreferExample {
            code: 409,
            example: Some("shortPaymentDenied"),
        },
    )
    .bearer(RECEP_TOKEN)
    .send_expect(409)
    .json();
    assert_eq!(err.code, ErrorCode::GuardViolation);
    assert_eq!(err.details.unwrap()["paid_cents"], 10000);
}
