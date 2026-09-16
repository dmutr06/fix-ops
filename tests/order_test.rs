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

#[test]
fn test_loyalty_discount_applied_to_labor_only() {
    let mut order = RepairOrder::new(3, "Bob Vance", LoyaltyTier::Silver);
    order.parts_cost = 100.0;
    order.labor_cost = 100.0;
    // Silver = 10% discount on labor ($100 * 0.10 = $10). Parts ($100) not discounted. Total = 190.
    assert_eq!(order.calculate_discount(), 10.0);
    assert_eq!(order.calculate_total(), 190.0);
}
