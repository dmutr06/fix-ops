//! Staff roles and the signed-in actor.

use crate::id::StaffId;

/// Staff role from SRS §2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Receptionist,
    Technician,
    Administrator,
}

impl Role {
    pub const ALL: [Self; 3] = [Self::Receptionist, Self::Technician, Self::Administrator];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Receptionist => "Receptionist",
            Self::Technician => "Technician",
            Self::Administrator => "Administrator",
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Role {
    type Err = UnknownRole;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|role| role.as_str() == s)
            .ok_or_else(|| UnknownRole(s.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown role {0:?}")]
pub struct UnknownRole(pub String);

/// The signed-in staff member performing an action. Every change is attributed to one actor (REQ-NF-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor {
    pub id: StaffId,
    pub role: Role,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_round_trips_through_its_name() {
        for role in Role::ALL {
            assert_eq!(role.as_str().parse::<Role>(), Ok(role));
        }
        assert!("Manager".parse::<Role>().is_err());
    }
}
