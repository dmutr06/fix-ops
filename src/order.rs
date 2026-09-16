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
        // Volume-based discount on gross order cost
        let gross_total = self.parts_cost + self.labor_cost;
        if gross_total >= 300.0 {
            30.0
        } else if gross_total >= 150.0 {
            15.0
        } else {
            0.0
        }
    }

    pub fn calculate_total(&self) -> f64 {
        match self.status {
            OrderStatus::Cancelled => Self::DIAGNOSTIC_FEE,
            _ => (self.parts_cost + self.labor_cost - self.calculate_discount()).max(0.0),
        }
    }
}
