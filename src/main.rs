use pis::order::{LoyaltyTier, RepairOrder};

fn main() {
    let mut order = RepairOrder::new(101, "Alex Chen", LoyaltyTier::Silver);
    order.parts_cost = 120.0;
    order.labor_cost = 80.0;

    println!("Order #{} for {}", order.id, order.client_name);
    println!("Total: ${:.2}", order.calculate_total());
}
