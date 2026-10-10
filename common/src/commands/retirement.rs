//! Native child observation for the shared, bounded retirement owner.
use crate::{
    retirement::{Admission, Observe, Pool},
    retirement_limits,
};
use std::{io, sync::OnceLock};
pub(super) fn reserve() -> io::Result<Admission> {
    static POOL: OnceLock<io::Result<Pool>> = OnceLock::new();
    POOL.get_or_init(|| {
        Pool::new(retirement_limits::configured(
            "BLENT_PROCESS_LIMIT",
            128,
            4096,
        )?)
    })
    .as_ref()
    .map_err(|error| io::Error::new(error.kind(), error.to_string()))?
    .reserve()
}
pub(super) struct SyncChild(pub std::process::Child);
impl Observe for SyncChild {
    fn poll_complete(&mut self) -> bool {
        matches!(self.0.try_wait(), Ok(Some(_)))
    }
}
pub(super) struct AsyncChild {
    pub child: tokio::process::Child,
    pub leases: Vec<Box<dyn Send + Sync>>,
    pub cleanup: Option<crate::retirement_work::Cleanup>,
}
impl Observe for AsyncChild {
    fn poll_complete(&mut self) -> bool {
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            return false;
        }
        if !self.leases.is_empty() {
            let leases = std::mem::take(&mut self.leases);
            self.cleanup = Some(crate::retirement_work::Cleanup::new(move || drop(leases)));
        }
        self.cleanup.as_mut().is_none_or(Observe::poll_complete)
    }
}
