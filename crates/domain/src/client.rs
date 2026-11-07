//! Clients and loyalty tiers (REQ-F-003, REQ-F-017).

use std::fmt;

use crate::audit::FieldChange;
use crate::error::InvalidField;
use crate::id::ClientId;

/// A client's name: trimmed and non-empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientName(String);

impl ClientName {
    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        let _ = raw;
        todo!()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A client's phone number: trimmed and non-empty. The SRS sets no format.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Phone(String);

impl Phone {
    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        let _ = raw;
        todo!()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An email address (`format: email` in the contract), trimmed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email(String);

impl Email {
    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        let _ = raw;
        todo!()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

macro_rules! display_as_str {
    ($($name:ty),+) => {$(
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    )+};
}

display_as_str!(ClientName, Phone, Email, LoyaltyTier);

/// Loyalty tier from SRS §3.2, set manually by an Administrator (REQ-F-017).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum LoyaltyTier {
    #[default]
    Standard,
    Silver,
    Gold,
}

impl LoyaltyTier {
    pub const ALL: [Self; 3] = [Self::Standard, Self::Silver, Self::Gold];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Silver => "Silver",
            Self::Gold => "Gold",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        let _ = raw;
        todo!()
    }
}

/// A stored client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    pub id: ClientId,
    pub name: ClientName,
    pub phone: Phone,
    pub email: Option<Email>,
    pub loyalty_tier: LoyaltyTier,
}

/// A client about to be created. New clients start at [`LoyaltyTier::Standard`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewClient {
    pub name: ClientName,
    pub phone: Phone,
    pub email: Option<Email>,
}

impl NewClient {
    /// Audit changes for the creation: every set field, including the starting tier.
    pub fn changes(&self) -> Vec<FieldChange> {
        todo!()
    }
}

/// A partial update. `None` keeps a field; `email: Some(None)` clears the email.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientPatch {
    pub name: Option<ClientName>,
    pub phone: Option<Phone>,
    pub email: Option<Option<Email>>,
}

impl Client {
    /// Applies `patch` and returns the updated client with one change per changed field.
    #[must_use]
    pub fn update(&self, patch: ClientPatch) -> (Self, Vec<FieldChange>) {
        let _ = patch;
        todo!()
    }

    /// Sets the loyalty tier. Order spend never calls this (REQ-F-017).
    #[must_use]
    pub fn set_loyalty_tier(&self, tier: LoyaltyTier) -> (Self, Vec<FieldChange>) {
        let _ = tier;
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bob() -> Client {
        Client {
            id: ClientId(10),
            name: ClientName::parse("Bob Vance").unwrap(),
            phone: Phone::parse("+1-555-0100").unwrap(),
            email: Some(Email::parse("bob@vance.org").unwrap()),
            loyalty_tier: LoyaltyTier::Standard,
        }
    }

    fn change(field: &'static str, previous: Option<&str>, new: Option<&str>) -> FieldChange {
        FieldChange::new(field, previous.map(Into::into), new.map(Into::into))
    }

    #[test]
    fn name_and_phone_are_trimmed_and_required() {
        assert_eq!(
            ClientName::parse("  Bob Vance ").unwrap().as_str(),
            "Bob Vance"
        );
        assert_eq!(
            Phone::parse(" +1-555-0100 ").unwrap().as_str(),
            "+1-555-0100"
        );
        for blank in ["", "   ", "\t\n"] {
            assert_eq!(ClientName::parse(blank).unwrap_err().field, "name");
            assert_eq!(Phone::parse(blank).unwrap_err().field, "phone");
        }
    }

    #[test]
    fn email_must_be_an_address() {
        for valid in [
            "bob@vance.org",
            "a.b+c@mail.example.co.uk",
            " bob@vance.org ",
        ] {
            assert_eq!(
                Email::parse(valid).unwrap().as_str(),
                valid.trim(),
                "{valid}"
            );
        }
        for invalid in [
            "",
            "   ",
            "bob",
            "bob@",
            "@vance.org",
            "bob@vance",
            "bob @vance.org",
            "bob@@vance.org",
            "bob@vance..org",
            "bob@.vance.org",
            "bob@vance.org.",
        ] {
            assert_eq!(
                Email::parse(invalid).unwrap_err().field,
                "email",
                "{invalid:?}"
            );
        }
    }

    #[test]
    fn loyalty_tier_parses_its_three_names_only() {
        for tier in LoyaltyTier::ALL {
            assert_eq!(LoyaltyTier::parse(tier.as_str()), Ok(tier));
        }
        assert_eq!(LoyaltyTier::default(), LoyaltyTier::Standard);
        for invalid in ["Platinum", "gold", "", " Gold"] {
            assert_eq!(
                LoyaltyTier::parse(invalid).unwrap_err().field,
                "loyalty_tier"
            );
        }
    }

    #[test]
    fn creation_audits_every_set_field_and_the_starting_tier() {
        let new = NewClient {
            name: ClientName::parse("Bob Vance").unwrap(),
            phone: Phone::parse("+1-555-0100").unwrap(),
            email: Some(Email::parse("bob@vance.org").unwrap()),
        };
        assert_eq!(
            new.changes(),
            vec![
                FieldChange::created("name", "Bob Vance"),
                FieldChange::created("phone", "+1-555-0100"),
                FieldChange::created("email", "bob@vance.org"),
                FieldChange::created("loyalty_tier", "Standard"),
            ]
        );

        let without_email = NewClient { email: None, ..new };
        assert!(
            without_email.changes().iter().all(|c| c.field != "email"),
            "an absent email is not a set field"
        );
    }

    #[test]
    fn update_records_only_changed_fields() {
        let client = bob();
        let (same, changes) = client.update(ClientPatch {
            phone: Some(Phone::parse("+1-555-0100").unwrap()),
            ..ClientPatch::default()
        });
        assert_eq!(same, client);
        assert_eq!(changes, []);

        let (updated, changes) = client.update(ClientPatch {
            phone: Some(Phone::parse("+1-555-0199").unwrap()),
            ..ClientPatch::default()
        });
        assert_eq!(updated.phone.as_str(), "+1-555-0199");
        assert_eq!(updated.name, client.name);
        assert_eq!(updated.email, client.email);
        assert_eq!(
            changes,
            vec![change("phone", Some("+1-555-0100"), Some("+1-555-0199"))]
        );
    }

    #[test]
    fn update_can_clear_and_set_the_email() {
        let (cleared, changes) = bob().update(ClientPatch {
            email: Some(None),
            ..ClientPatch::default()
        });
        assert_eq!(cleared.email, None);
        assert_eq!(changes, vec![change("email", Some("bob@vance.org"), None)]);

        let (set, changes) = cleared.update(ClientPatch {
            email: Some(Some(Email::parse("b@v.org").unwrap())),
            ..ClientPatch::default()
        });
        assert_eq!(set.email.as_ref().map(Email::as_str), Some("b@v.org"));
        assert_eq!(changes, vec![change("email", None, Some("b@v.org"))]);
    }

    #[test]
    fn tier_change_is_one_audited_field() {
        let (gold, changes) = bob().set_loyalty_tier(LoyaltyTier::Gold);
        assert_eq!(gold.loyalty_tier, LoyaltyTier::Gold);
        assert_eq!(gold.name, bob().name);
        assert_eq!(
            changes,
            vec![change("loyalty_tier", Some("Standard"), Some("Gold"))]
        );

        let (same, changes) = gold.set_loyalty_tier(LoyaltyTier::Gold);
        assert_eq!(same, gold);
        assert_eq!(changes, []);
    }
}
