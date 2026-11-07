//! Typed record identifiers, so a client id can never be passed where an order id is expected.

macro_rules! id {
    ($($(#[$doc:meta])* $name:ident),+ $(,)?) => {$(
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub i64);

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    )+};
}

id! {
    /// Identifier of a staff account.
    StaffId,
    /// Identifier of a client.
    ClientId,
    /// Identifier of a device.
    DeviceId,
    /// Identifier of a repair order.
    OrderId,
}
