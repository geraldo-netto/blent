//! One manual intent fixes its first effective target before any invitation.
use super::{PeerId, Target};
use anyhow::{ensure, Result};
use std::{future::Future, sync::OnceLock};

// Immutable requested preferences and the committed effective target have distinct
// roles. Neither grants authority; the native adapter revalidates the original.
enum Requested {
    Adb(Option<String>),
    Usb(PeerId),
}
enum Resolved {
    Adb(String),
    Usb(PeerId),
}

pub struct BoundTarget(Resolved);
impl BoundTarget {
    /// Borrow only within this binding's lifetime; USB returns its typed identity.
    pub fn target(&self) -> Target<'_> {
        match &self.0 {
            Resolved::Adb(serial) => Target::Adb(Some(serial)),
            Resolved::Usb(peer) => Target::Usb(peer.clone()),
        }
    }
}

/// One owner per explicit manual intent. No Clone/reset/reselection operation.
/// Concurrent lookups may finish in either order; the first successful binding
/// wins, and all successful callers receive that same immutable original.
/// The lookup callback grants no media authority and must independently retain
/// any native work on cancellation. A cancelled, uncommitted lookup cannot
/// publish a late binding. Existing bindings perform no further lookup.
/// Storage: one requested selector and one committed selector; O(serial length),
/// or fixed-size peer identities. Input borrows never escape construction.
pub struct OriginalTarget {
    requested: Requested,
    original: OnceLock<BoundTarget>,
}
impl OriginalTarget {
    pub fn new(target: Target<'_>) -> Result<Self> {
        let requested = match target {
            Target::Adb(serial) => {
                ensure!(serial != Some(""), "missing tablet serial");
                Requested::Adb(serial.map(str::to_owned))
            }
            Target::Usb(peer) => Requested::Usb(peer),
        };
        Ok(Self {
            requested,
            original: OnceLock::new(),
        })
    }

    pub async fn bind<F, Fut>(&self, resolve_adb: F) -> Result<&BoundTarget>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<String>>,
    {
        if let Some(original) = self.original.get() {
            return Ok(original);
        }
        let resolved = match &self.requested {
            Requested::Adb(Some(serial)) => Resolved::Adb(serial.clone()),
            Requested::Adb(None) => {
                let serial = resolve_adb().await?;
                ensure!(!serial.is_empty(), "missing tablet serial");
                Resolved::Adb(serial)
            }
            Requested::Usb(peer) => Resolved::Usb(peer.clone()),
        };
        Ok(self.original.get_or_init(|| BoundTarget(resolved)))
    }
}

#[cfg(test)]
mod tests;
