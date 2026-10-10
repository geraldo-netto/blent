//! An admitted native owner survives caller cancellation until route retirement.
//! The address lease prevents a later in-process attempt overtaking old cleanup.
use crate::usb::{Adb, Commands};
use anyhow::{ensure, Context, Result};
use blent_config::{
    retirement::{Completion, Observe, Pool},
    retirement_work::Thread,
};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
use tokio::sync::oneshot;

struct Owners {
    pool: Pool,
    addresses: Arc<Mutex<HashSet<String>>>,
}
impl Owners {
    fn new(capacity: usize) -> std::io::Result<Self> {
        Ok(Self {
            pool: Pool::new(capacity)?,
            addresses: Default::default(),
        })
    }
    fn shared() -> Result<&'static Self> {
        static OWNERS: OnceLock<std::io::Result<Owners>> = OnceLock::new();
        OWNERS
            .get_or_init(|| {
                let n = blent_config::retirement_limits::configured("BLENT_WIFI_ATTEMPTS", 8, 64)?;
                Self::new(n)
            })
            .as_ref()
            .map_err(|e| anyhow::anyhow!("Wi-Fi attempt owner unavailable: {e}"))
    }
    fn reserve(&self, address: &str) -> Result<(blent_config::retirement::Admission, Address)> {
        ensure!(super::valid_address(address), "Invalid Wi-Fi address");
        let slot = self.pool.reserve()?;
        let mut addresses = self.addresses.lock().unwrap();
        ensure!(
            addresses.insert(address.into()),
            "Wi-Fi address still active or retiring"
        );
        Ok((
            slot,
            Address {
                owners: self.addresses.clone(),
                address: address.into(),
            },
        ))
    }
}
struct Address {
    owners: Arc<Mutex<HashSet<String>>>,
    address: String,
}
impl Drop for Address {
    fn drop(&mut self) {
        self.owners.lock().unwrap().remove(&self.address);
    }
}
struct Retiring {
    thread: Thread,
    _address: Address,
}
impl Observe for Retiring {
    fn poll_complete(&mut self) -> bool {
        self.thread.poll_complete()
    }
}

pub(super) struct Attempt {
    decision: Option<oneshot::Sender<bool>>,
    completion: Completion,
}
impl Attempt {
    pub(super) async fn start<C: Commands + Clone + 'static>(
        adb: &Adb<C>,
        address: &str,
        connect: bool,
    ) -> Result<Self> {
        Self::with_owner(Owners::shared()?, adb.clone(), address, connect).await
    }
    async fn with_owner<C: Commands + 'static>(
        owners: &Owners,
        adb: Adb<C>,
        address: &str,
        connect: bool,
    ) -> Result<Self> {
        let (slot, lease) = owners.reserve(address)?;
        let address = address.to_owned();
        let completion = slot.completion();
        let (decision, chosen) = oneshot::channel();
        let (ready, reply) = oneshot::channel();
        let handle = std::thread::Builder::new()
            .name("blent-wifi-attempt".into())
            .spawn(move || {
                run(adb, address, connect, ready, chosen);
            })?;
        slot.retain(Retiring {
            thread: Thread::new(handle),
            _address: lease,
        });
        let attempt = Self {
            decision: Some(decision),
            completion,
        };
        match reply
            .await
            .context("Wi-Fi attempt worker failed")
            .and_then(|r| r)
        {
            Ok(()) => Ok(attempt),
            Err(error) => {
                let _ = attempt.finish(false).await;
                Err(error)
            }
        }
    }
    pub(super) async fn finish(mut self, keep: bool) -> Result<()> {
        let _ = self.decision.take().unwrap().send(keep);
        tokio::time::timeout(Duration::from_secs(6), async {
            while !self.completion.is_complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .context("Wi-Fi retirement incomplete; address and capacity retained")
    }
}
fn run<C: Commands>(
    adb: Adb<C>,
    address: String,
    connect: bool,
    ready: oneshot::Sender<Result<()>>,
    chosen: oneshot::Receiver<bool>,
) {
    // Native lease acquisition is confined to this admitted thread. Its guard
    // outlives connect, caller decision, and any compensating disconnect.
    let _lease = match adb.0.route_lease(&address) {
        Ok(lease) => lease,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();
    let runtime = match runtime {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(error.into()));
            return;
        }
    };
    runtime.block_on(execute_attempt(adb, address, connect, ready, chosen));
}
async fn execute_attempt<C: Commands>(
    adb: Adb<C>,
    address: String,
    connect: bool,
    ready: oneshot::Sender<Result<()>>,
    chosen: oneshot::Receiver<bool>,
) {
    if ready.is_closed() {
        return;
    }
    let result = command(&adb, &address, connect).await;
    match result {
        Ok(created) => {
            let _ = ready.send(Ok(()));
            if !chosen.await.unwrap_or(false) && created {
                if let Err(error) = command(&adb, &address, false).await {
                    tracing::warn!("Wi-Fi route cleanup failed: {error:#}");
                }
            }
        }
        Err(error) => {
            let _ = ready.send(Err(error));
        }
    }
}
async fn command<C: Commands>(adb: &Adb<C>, address: &str, connect: bool) -> Result<bool> {
    let operation = if connect { "connect" } else { "disconnect" };
    let bytes = adb
        .checked(vec![operation.into(), address.into()], None)
        .await?;
    if !connect {
        return Ok(false);
    }
    ensure!(
        super::policy::connected(&bytes, address),
        "ADB did not confirm network connection"
    );
    Ok(String::from_utf8_lossy(&bytes).trim() != format!("already connected to {address}"))
}

#[cfg(test)]
pub(super) mod tests;
