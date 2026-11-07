//! Settlement of a repair order (SRS §3.2, REQ-F-015, REQ-F-016).
//!
//! `labor_discount_cents` and `settlement_cents` are always derived here, never entered.

use crate::client::LoyaltyTier;
use crate::money::Cents;
use crate::order::{OrderStatus, RepairOrder};

/// The labor discount and the amount owed for an order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Settlement {
    pub labor_discount: Cents,
    pub total: Cents,
}

impl Settlement {
    /// Nothing owed and no discount.
    pub const NONE: Self = Self {
        labor_discount: Cents::ZERO,
        total: Cents::ZERO,
    };
}

/// The settlement an order shows now: the stored one once the order is terminal, otherwise
/// the amount computed from what has been incurred so far with the client's current `tier`.
pub fn current_settlement(order: &RepairOrder, _tier: LoyaltyTier) -> Settlement {
    if let Some(settlement) = order.final_settlement {
        return settlement;
    }
    match order.status {
        OrderStatus::Received | OrderStatus::Diagnosing => Settlement::NONE,
        status => todo!("settlement of an open {status} order"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::tests::received;

    /// SRS §3.2: before diagnostic findings are saved nothing is owed.
    #[test]
    fn a_received_order_owes_nothing() {
        assert_eq!(
            current_settlement(&received(), LoyaltyTier::Gold),
            Settlement::NONE
        );
    }
}
