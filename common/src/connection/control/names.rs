//! Bounded USB descriptor hints, never device identity or pairing authority.
use anyhow::{ensure, Result};

pub const MAX_NAME_BYTES: usize = 256;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeviceNames {
    manufacturer: String,
    model: String,
}

impl DeviceNames {
    /// Empty strings explicitly mean the descriptor is unavailable.
    pub fn new(manufacturer: String, model: String) -> Result<Self> {
        validate(&manufacturer)?;
        validate(&model)?;
        Ok(Self {
            manufacturer,
            model,
        })
    }

    pub fn manufacturer(&self) -> &str {
        &self.manufacturer
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

fn validate(value: &str) -> Result<()> {
    ensure!(value.len() <= MAX_NAME_BYTES, "USB name exceeds capacity");
    ensure!(value.trim() == value, "USB name has surrounding whitespace");
    ensure!(
        !value.chars().any(unsafe_character),
        "invalid USB name character"
    );
    Ok(())
}

pub fn unsafe_character(value: char) -> bool {
    value.is_control()
        || matches!(value, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1336_names_bound_each_field_independently() {
        for length in [0, 1, 255, 256, 257, 512] {
            let value = "x".repeat(length);
            assert_eq!(
                DeviceNames::new(value.clone(), String::new()).is_ok(),
                length <= MAX_NAME_BYTES
            );
            assert_eq!(
                DeviceNames::new(String::new(), value).is_ok(),
                length <= MAX_NAME_BYTES
            );
        }
        let names = DeviceNames::new("メーカー".into(), "平板".into()).unwrap();
        assert_eq!(names.manufacturer(), "メーカー");
        assert_eq!(names.model(), "平板");
    }

    #[test]
    fn t1336_control_and_direction_overrides_cannot_spoof_rows() {
        for value in [" x", "x ", "x\ny", "x\0y", "x\u{202e}y", "x\u{2067}y"] {
            assert!(DeviceNames::new(value.into(), String::new()).is_err());
            assert!(DeviceNames::new(String::new(), value.into()).is_err());
        }
        for code in 0..=255 {
            let value = char::from_u32(code).unwrap();
            assert_eq!(unsafe_character(value), value.is_control());
        }
    }
}
