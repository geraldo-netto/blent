//! Independent local native-observer wire schema; no X11 or OS types.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayObservation {
    pub supported: bool,
    pub on: Option<bool>,
}
impl DisplayObservation {
    pub fn validate(self) -> Result<()> {
        ensure!(
            self.supported || self.on.is_none(),
            "unsupported display source supplied state"
        );
        Ok(())
    }
    pub fn encode(self) -> Result<Vec<u8>> {
        self.validate()?;
        Ok(serde_json::to_vec(&self)?)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= 256, "oversized display observation");
        let value: Self = serde_json::from_slice(bytes)?;
        value.validate()?;
        Ok(value)
    }
}
