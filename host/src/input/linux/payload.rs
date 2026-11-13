//! T1439: one native payload remains owned through its original mapping commands.
use super::{lifetime, InjectDevices, TOUCH_DEVICES};
use crate::native_commands::ObservedCommands;
use blent_config::retirement::{Admission, Completion, Observe, Pool};
use blent_config::retirement_work::Cleanup;
use futures_util::FutureExt;
use std::{
    io,
    sync::{atomic::Ordering, Arc, OnceLock},
};

fn pool() -> io::Result<&'static Pool> {
    static POOL: OnceLock<io::Result<Pool>> = OnceLock::new();
    initialized_pool(&POOL, lifetime::MAX_BACKENDS)
}

fn initialized_pool(storage: &OnceLock<io::Result<Pool>>, capacity: usize) -> io::Result<&Pool> {
    storage
        .get_or_init(|| Pool::new(capacity))
        .as_ref()
        .map_err(|error| io::Error::new(error.kind(), error.to_string()))
}

struct Context {
    commands: Arc<ObservedCommands>,
    osk: lifetime::OskObservations,
    runtime: tokio::runtime::Handle,
}

pub(super) struct Live {
    admission: Admission,
    context: Context,
}
impl Live {
    pub(super) fn new(
        commands: Arc<ObservedCommands>,
        osk: lifetime::OskObservations,
    ) -> io::Result<Self> {
        Ok(Self {
            admission: pool()?.reserve()?,
            context: Context {
                commands,
                osk,
                runtime: tokio::runtime::Handle::current(),
            },
        })
    }
    pub(super) fn completion(&self) -> Completion {
        self.admission.completion()
    }
    pub(super) fn retire(self, devices: InjectDevices, registered: bool) -> Completion {
        let proof = self.completion();
        self.admission.retain(Retiring {
            context: self.context,
            devices: Some(devices),
            cleanup: None,
            registered,
            restoration_started: false,
            restoration: None,
            restoration_failed: false,
        });
        proof
    }
}

struct Retiring {
    context: Context,
    devices: Option<InjectDevices>,
    cleanup: Option<Cleanup>,
    registered: bool,
    restoration_started: bool,
    restoration: Option<tokio::task::JoinHandle<()>>,
    restoration_failed: bool,
}
impl Retiring {
    fn restored(&mut self) -> bool {
        if let Some(task) = self.restoration.as_mut() {
            let Some(result) = task.now_or_never() else {
                return false;
            };
            self.restoration.take();
            self.restoration_failed = result.is_err();
            if self.restoration_failed {
                tracing::warn!("OSK cleanup task failed; payload admission remains quarantined");
            }
        }
        !self.restoration_failed
    }
    fn destroyed(&mut self) -> bool {
        if !self.context.commands.complete() {
            return false;
        }
        if self.cleanup.is_none() {
            let Some(devices) = self.devices.take() else {
                return true;
            };
            if devices.count() == 0 {
                return true;
            }
            self.cleanup = Some(Cleanup::new(move || drop(devices)));
        }
        self.cleanup.as_mut().unwrap().poll_complete()
    }
}
impl Observe for Retiring {
    fn poll_complete(&mut self) -> bool {
        if !self.destroyed() {
            return false;
        }
        if !self.restoration_started {
            self.restoration_started = true;
            self.restoration = self.context.restore(std::mem::take(&mut self.registered));
        }
        self.restored() && self.context.osk.lock().unwrap().complete()
    }
}
impl Context {
    fn restore(&self, registered: bool) -> Option<tokio::task::JoinHandle<()>> {
        if !registered {
            return None;
        }
        TOUCH_DEVICES.fetch_sub(1, Ordering::SeqCst);
        let osk = self.osk.clone();
        // The payload admission funds exactly one restoration execution after
        // native destruction. Its owner retains and joins this original task;
        // cleanup never competes for a second shared work-admission slot.
        Some(self.runtime.spawn(async move {
            crate::osk::sync_touch_state(&TOUCH_DEVICES, &|proof| {
                lifetime::observe_osk(&osk, proof)
            })
            .await;
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn complete(proof: &Completion) {
        while !proof.is_complete() {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    }

    #[tokio::test]
    async fn t1439_one_payload_slot_rejects_overlap_and_recovers_without_native_devices() {
        let lifetime = lifetime::Lifetime::new().unwrap();
        let live = lifetime.admit_payload().unwrap();
        assert!(lifetime.admit_payload().is_err());
        let proof = live.retire(InjectDevices::empty(), false);
        complete(&proof).await;
        let fresh = lifetime.admit_payload().unwrap();
        let before_effects = fresh.completion();
        drop(fresh);
        assert!(before_effects.is_complete());
        let proof = lifetime.completion();
        drop(lifetime);
        complete(&proof).await;
    }

    fn retirement(task: tokio::task::JoinHandle<()>) -> Retiring {
        Retiring {
            context: Context {
                commands: Arc::new(ObservedCommands::default()),
                osk: Default::default(),
                runtime: tokio::runtime::Handle::current(),
            },
            devices: Some(InjectDevices::empty()),
            cleanup: None,
            registered: false,
            restoration_started: true,
            restoration: Some(task),
            restoration_failed: false,
        }
    }

    #[tokio::test]
    async fn t1439_cleanup_observes_original_task_join_and_retains_failure() {
        let (release, held) = tokio::sync::oneshot::channel::<()>();
        let mut original = retirement(tokio::spawn(async move { held.await.unwrap() }));
        assert!(!original.poll_complete());
        release.send(()).unwrap();
        while !original.poll_complete() {
            tokio::task::yield_now().await;
        }
        assert!(original.poll_complete());
        let failed = tokio::spawn(std::future::pending::<()>());
        failed.abort();
        let mut cancelled = retirement(failed);
        while !cancelled.restoration_failed {
            assert!(!cancelled.poll_complete());
            tokio::task::yield_now().await;
        }
        assert!(!cancelled.poll_complete());
        assert!(cancelled.restoration.is_none());
    }

    #[test]
    fn t1439_payload_pool_rejects_initialization_failure_without_replacing_its_grant() {
        let failed = OnceLock::new();
        assert_eq!(
            initialized_pool(&failed, 0).err().unwrap().kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(initialized_pool(&failed, 1).is_err());
        let storage = OnceLock::new();
        let first = initialized_pool(&storage, 1).unwrap().reserve().unwrap();
        assert!(initialized_pool(&storage, 2).unwrap().reserve().is_err());
        drop(first);
        assert!(initialized_pool(&storage, 1).unwrap().reserve().is_ok());
    }
}
