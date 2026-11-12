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
mod cleanup;

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
        let address = super::policy::endpoint_key(address);
        let slot = self.pool.reserve()?;
        let mut addresses = self.addresses.lock().unwrap();
        ensure!(
            addresses.insert(address.clone()),
            "Wi-Fi address still active or retiring"
        );
        Ok((
            slot,
            Address {
                owners: self.addresses.clone(),
                address,
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
    ownership: Arc<RouteOwnership>,
}
#[derive(Default)]
struct RouteOwnership {
    native: Mutex<Option<Box<dyn Send>>>,
    settled: std::sync::atomic::AtomicBool,
}
impl Observe for Retiring {
    fn poll_complete(&mut self) -> bool {
        self.thread.poll_complete()
            && self
                .ownership
                .settled
                .load(std::sync::atomic::Ordering::Acquire)
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
        let ownership = Arc::new(RouteOwnership::default());
        let worker = ownership.clone();
        let handle = std::thread::Builder::new()
            .name("blent-wifi-attempt".into())
            .spawn(move || {
                run(adb, address, connect, ready, chosen, &worker);
                worker
                    .settled
                    .store(true, std::sync::atomic::Ordering::Release);
            })?;
        slot.retain(Retiring {
            thread: Thread::new(handle),
            _address: lease,
            ownership,
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
    ownership: &RouteOwnership,
) {
    // Native lease acquisition is confined to this admitted thread. Its guard
    // outlives connect, caller decision, and any compensating disconnect.
    let lease = match adb.0.route_lease(&address) {
        Ok(lease) => lease,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    *ownership.native.lock().unwrap() = Some(lease);
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
    use blent_config::adb_reverse::temporary::Creation;
    let mut creation = Creation::NotStarted;
    let result = if connect {
        connect_route(&adb, &address, &mut creation).await
    } else {
        cleanup::retire(&adb, &address, Creation::Acknowledged).await;
        Ok(false)
    };
    match result {
        Ok(created) => {
            let _ = ready.send(Ok(()));
            if !chosen.await.unwrap_or(false) && created {
                cleanup::retire(&adb, &address, creation).await;
            }
        }
        Err(error) => {
            let _ = ready.send(Err(error));
            if creation != Creation::NotStarted {
                cleanup::retire(&adb, &address, creation).await;
            }
        }
    }
}
async fn connect_route<C: Commands>(
    adb: &Adb<C>,
    address: &str,
    creation: &mut blent_config::adb_reverse::temporary::Creation,
) -> Result<bool> {
    use blent_config::adb_reverse::temporary::Creation;
    if let Some(ready) = cleanup::observe(adb, address).await? {
        ensure!(ready, "Existing network ADB transport is not ready");
        return Ok(false); // Existing transports never become compensation targets.
    }
    *creation = Creation::Uncertain;
    let command = adb
        .0
        .execute_route(vec!["connect".into(), address.into()])
        .await;
    if !command.started {
        *creation = Creation::NotStarted;
    }
    let output = command.output?;
    ensure!(
        output.status.success() && output.stdout.len() <= 65536,
        "ADB network connection outcome unconfirmed"
    );
    ensure!(
        super::policy::connected(&output.stdout, address),
        "ADB did not confirm network connection"
    );
    *creation = Creation::Acknowledged;
    Ok(true)
}

#[cfg(test)]
mod rollback_tests;
#[cfg(test)]
pub(super) mod tests;

#[cfg(test)]
mod creation_tests;
