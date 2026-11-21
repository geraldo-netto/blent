//! Owned mapping names; no protocol/backend types enter this contract.
use std::io;
pub(crate) struct Mapping {
    pub(super) device: String,
    pub(super) output: String,
}
impl Mapping {
    pub(crate) fn new(device: &str, output: &str) -> io::Result<Self> {
        if [device, output]
            .iter()
            .any(|name| name.is_empty() || name.len() > 4096 || name.contains('\0'))
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            device: device.to_owned(),
            output: output.to_owned(),
        })
    }
}
