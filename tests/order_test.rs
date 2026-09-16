use pis::order::{LoyaltyTier, OrderStatus, RepairOrder};

#[test]
fn test_order_creation() {
    let order = RepairOrder::new(1, "John Doe", LoyaltyTier::Standard);
    assert_eq!(order.id, 1);
    assert_eq!(order.status, OrderStatus::Received);
}

#[test]
fn test_cancelled_order_charges_diagnostic_fee() {
    let mut order = RepairOrder::new(2, "Jane Smith", LoyaltyTier::Standard);
    order.status = OrderStatus::Cancelled;
    assert_eq!(order.calculate_total(), RepairOrder::DIAGNOSTIC_FEE);
}
