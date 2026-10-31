#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoyaltyTier {
    Standard,
    Silver,
    Gold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    Received,
    Diagnosing,
    InProgress,
    Ready,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct RepairOrder {
    pub id: u32,
    pub client_name: String,
    pub loyalty_tier: LoyaltyTier,
    pub status: OrderStatus,
    pub parts_cost: f64,
    pub labor_cost: f64,
}

impl RepairOrder {
    pub const DIAGNOSTIC_FEE: f64 = 30.0;

    pub fn new(id: u32, client_name: &str, loyalty_tier: LoyaltyTier) -> Self {
        Self {
            id,
            client_name: client_name.to_string(),
            loyalty_tier,
            status: OrderStatus::Received,
            parts_cost: 0.0,
            labor_cost: 0.0,
        }
    }

    pub fn calculate_discount(&self) -> f64 {
        let rate = match self.loyalty_tier {
            LoyaltyTier::Standard => 0.0,
            LoyaltyTier::Silver => 0.10,
            LoyaltyTier::Gold => 0.20,
        };
        self.labor_cost * rate
    }

    pub fn calculate_total(&self) -> f64 {
        match self.status {
            OrderStatus::Cancelled => Self::DIAGNOSTIC_FEE,
            _ => (self.parts_cost + self.labor_cost - self.calculate_discount()).max(0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LoyaltyTier, OrderStatus, RepairOrder};

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
}
