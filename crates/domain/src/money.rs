//! Money in whole USD cents (SRS §3.2, ADD-002 §4). No floats anywhere.

use std::fmt;

use crate::error::InvalidField;

/// A non-negative amount of whole cents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cents(i64);

impl Cents {
    pub const ZERO: Self = Self(0);

    /// Accepts `value` when it is not negative. `field` names the contract field for the error.
    pub fn new(field: &'static str, value: i64) -> Result<Self, InvalidField> {
        let _ = (field, value);
        todo!()
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

impl fmt::Display for Cents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = f;
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_amounts_are_rejected_naming_the_field() {
        let err = Cents::new("used_parts_cents", -1).unwrap_err();
        assert_eq!(err.field, "used_parts_cents");
        assert!(Cents::new("x", i64::MIN).is_err());
    }

    #[test]
    fn zero_and_positive_amounts_are_kept_exactly() {
        assert_eq!(Cents::new("x", 0), Ok(Cents::ZERO));
        assert_eq!(Cents::new("x", 1).map(Cents::get), Ok(1));
        assert_eq!(Cents::new("x", i64::MAX).map(Cents::get), Ok(i64::MAX));
    }

    #[test]
    fn renders_as_whole_cents() {
        assert_eq!(Cents::ZERO.to_string(), "0");
        assert_eq!(Cents::new("x", 19_000).unwrap().to_string(), "19000");
    }
}
