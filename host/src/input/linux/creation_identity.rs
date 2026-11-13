//! X11 creation names are distinct across payloads; KWin keeps persistent names.
use super::DeviceIdentity;
use blent_config::credentials::{system_entropy, Entropy};
use std::{io, sync::Mutex};

struct Sequence {
    namespace: u128,
    next: u64,
}
impl Sequence {
    fn new(entropy: Entropy) -> io::Result<Self> {
        let mut bytes = [0; 16];
        entropy(&mut bytes).map_err(io::Error::other)?;
        Ok(Self {
            namespace: u128::from_le_bytes(bytes),
            next: 0,
        })
    }
    fn issue(&mut self) -> io::Result<Stamp> {
        let serial = self.next;
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| io::Error::other("X11 input creation identity sequence exhausted"))?;
        Ok(Stamp {
            namespace: self.namespace,
            serial,
        })
    }
}
struct Stamp {
    namespace: u128,
    serial: u64,
}
impl Stamp {
    fn name(&self, base: &str) -> String {
        format!("{base} @{:032x}-{:016x}", self.namespace, self.serial)
    }
    fn identity(&self, base: &DeviceIdentity) -> DeviceIdentity {
        DeviceIdentity {
            touch: self.name(&base.touch),
            pen: self.name(&base.pen),
            pointer: self.name(&base.pointer),
            product_touch: base.product_touch,
            product_pen: base.product_pen,
            product_pointer: base.product_pointer,
        }
    }
}
fn issue_using(storage: &Mutex<Option<Sequence>>, entropy: Entropy) -> io::Result<Stamp> {
    let mut source = storage
        .lock()
        .map_err(|_| io::Error::other("X11 input creation identity owner poisoned"))?;
    if source.is_none() {
        *source = Some(Sequence::new(entropy)?);
    }
    source.as_mut().unwrap().issue()
}
fn issue() -> io::Result<Stamp> {
    static SOURCE: Mutex<Option<Sequence>> = Mutex::new(None);
    issue_using(&SOURCE, system_entropy)
}
fn identity_using(
    base: &DeviceIdentity,
    session: &str,
    stamp: impl FnOnce() -> io::Result<Stamp>,
) -> io::Result<DeviceIdentity> {
    if session != "x11" {
        return Ok(base.clone());
    }
    Ok(stamp()?.identity(base))
}
pub(super) fn for_session(base: &DeviceIdentity) -> io::Result<DeviceIdentity> {
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    identity_using(base, &session, issue)
}

#[cfg(test)]
mod tests;
