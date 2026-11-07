//! Serviced devices (REQ-F-004).

use std::fmt;

use crate::audit::FieldChange;
use crate::error::InvalidField;
use crate::id::{ClientId, DeviceId};

/// Kind of device from the glossary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    Desktop,
    Laptop,
    Server,
}

impl DeviceType {
    pub const ALL: [Self; 3] = [Self::Desktop, Self::Laptop, Self::Server];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "Desktop",
            Self::Laptop => "Laptop",
            Self::Server => "Server",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        let _ = raw;
        todo!()
    }
}

/// A device serial number: trimmed and non-empty (BDD-002).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SerialNumber(String);

impl SerialNumber {
    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        let _ = raw;
        todo!()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The symptoms the client reported: trimmed and non-empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symptoms(String);

impl Symptoms {
    pub fn parse(raw: &str) -> Result<Self, InvalidField> {
        let _ = raw;
        todo!()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for SerialNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Symptoms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A stored device. It always belongs to one client, which never changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: DeviceId,
    pub client_id: ClientId,
    pub device_type: DeviceType,
    pub serial_number: SerialNumber,
    pub reported_symptoms: Symptoms,
}

/// A device about to be created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDevice {
    pub client_id: ClientId,
    pub device_type: DeviceType,
    pub serial_number: SerialNumber,
    pub reported_symptoms: Symptoms,
}

impl NewDevice {
    /// Audit changes for the creation: every field.
    pub fn changes(&self) -> Vec<FieldChange> {
        todo!()
    }
}

/// A partial update. `None` keeps a field.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DevicePatch {
    pub device_type: Option<DeviceType>,
    pub serial_number: Option<SerialNumber>,
    pub reported_symptoms: Option<Symptoms>,
}

impl Device {
    /// Applies `patch` and returns the updated device with one change per changed field.
    #[must_use]
    pub fn update(&self, patch: DevicePatch) -> (Self, Vec<FieldChange>) {
        let _ = patch;
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn laptop() -> Device {
        Device {
            id: DeviceId(20),
            client_id: ClientId(10),
            device_type: DeviceType::Laptop,
            serial_number: SerialNumber::parse("SN-THINKPAD-9001").unwrap(),
            reported_symptoms: Symptoms::parse("No power on AC adapter").unwrap(),
        }
    }

    #[test]
    fn device_type_parses_its_three_names_only() {
        for kind in DeviceType::ALL {
            assert_eq!(DeviceType::parse(kind.as_str()), Ok(kind));
        }
        for invalid in ["Phone", "laptop", ""] {
            assert_eq!(DeviceType::parse(invalid).unwrap_err().field, "device_type");
        }
    }

    #[test]
    fn serial_number_and_symptoms_are_trimmed_and_required() {
        assert_eq!(SerialNumber::parse(" SN-1 ").unwrap().as_str(), "SN-1");
        assert_eq!(Symptoms::parse(" No power ").unwrap().as_str(), "No power");
        for blank in ["", "   ", "\t\n"] {
            assert_eq!(
                SerialNumber::parse(blank).unwrap_err().field,
                "serial_number"
            );
            assert_eq!(
                Symptoms::parse(blank).unwrap_err().field,
                "reported_symptoms"
            );
        }
    }

    #[test]
    fn creation_audits_every_field() {
        let new = NewDevice {
            client_id: ClientId(10),
            device_type: DeviceType::Laptop,
            serial_number: SerialNumber::parse("SN-THINKPAD-9001").unwrap(),
            reported_symptoms: Symptoms::parse("No power on AC adapter").unwrap(),
        };
        assert_eq!(
            new.changes(),
            vec![
                FieldChange::created("client_id", 10),
                FieldChange::created("device_type", "Laptop"),
                FieldChange::created("serial_number", "SN-THINKPAD-9001"),
                FieldChange::created("reported_symptoms", "No power on AC adapter"),
            ]
        );
    }

    #[test]
    fn update_records_only_changed_fields() {
        let device = laptop();
        let (same, changes) = device.update(DevicePatch {
            device_type: Some(DeviceType::Laptop),
            ..DevicePatch::default()
        });
        assert_eq!(same, device);
        assert_eq!(changes, []);

        let (updated, changes) = device.update(DevicePatch {
            reported_symptoms: Some(Symptoms::parse("Battery LED blinks").unwrap()),
            ..DevicePatch::default()
        });
        assert_eq!(updated.reported_symptoms.as_str(), "Battery LED blinks");
        assert_eq!(updated.serial_number, device.serial_number);
        assert_eq!(updated.client_id, device.client_id);
        assert_eq!(
            changes,
            vec![FieldChange::new(
                "reported_symptoms",
                Some("No power on AC adapter".into()),
                Some("Battery LED blinks".into())
            )]
        );
    }
}
