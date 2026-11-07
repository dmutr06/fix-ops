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

/// The labor discount rate of a tier in percent: Standard 0, Silver 10, Gold 20.
pub const fn discount_percent(tier: LoyaltyTier) -> i64 {
    match tier {
        LoyaltyTier::Standard => 0,
        LoyaltyTier::Silver => 10,
        LoyaltyTier::Gold => 20,
    }
}

/// The tier discount on `labor`, rounded to whole cents with an exact half cent rounded up.
pub fn labor_discount(labor: Cents, tier: LoyaltyTier) -> Cents {
    let rate = discount_percent(tier);
    let value = labor.get();
    Cents::new(
        "labor_discount_cents",
        (value / 100) * rate + ((value % 100) * rate + 50) / 100,
    )
    .unwrap_or_default()
}

/// The total of a repair: used parts plus completed labor minus the labor-only discount.
pub fn repair_total(used_parts: Cents, completed_labor: Cents, tier: LoyaltyTier) -> Settlement {
    let labor_discount = labor_discount(completed_labor, tier);
    let total = used_parts
        .get()
        .saturating_add(completed_labor.get() - labor_discount.get());
    Settlement {
        labor_discount,
        total: Cents::new("settlement_cents", total).unwrap_or_default(),
    }
}

/// The settlement an order shows now: the stored one once the order is terminal, otherwise
/// the amount computed from what has been incurred so far with the client's current `tier`.
pub fn current_settlement(order: &RepairOrder, tier: LoyaltyTier) -> Settlement {
    if let Some(settlement) = order.final_settlement {
        return settlement;
    }
    match order.status {
        OrderStatus::InProgress | OrderStatus::Ready => {
            repair_total(order.used_parts, order.completed_labor, tier)
        }
        OrderStatus::Received
        | OrderStatus::Diagnosing
        | OrderStatus::Completed
        | OrderStatus::Cancelled => Settlement::NONE,
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
    #[test]
    fn repair_pricing_discounts_only_labor_and_rounds_half_up() {
        let cents = |n| Cents::new("amount", n).unwrap();
        assert_eq!(
            repair_total(cents(10000), cents(10000), LoyaltyTier::Silver),
            Settlement {
                labor_discount: cents(1000),
                total: cents(19000)
            }
        );
        assert_eq!(
            repair_total(cents(0), cents(5), LoyaltyTier::Silver),
            Settlement {
                labor_discount: cents(1),
                total: cents(4)
            }
        );
        assert_eq!(
            labor_discount(cents(i64::MAX), LoyaltyTier::Gold).get(),
            1_844_674_407_370_955_161
        );
    }
}
