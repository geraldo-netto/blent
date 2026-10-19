//! Native auxiliary observation composition; domain work consumes only publications.
use crate::{
    activity_follow::{Following, Settings},
    owned_task::Task,
};
use anyhow::{Context, Result};
use blent_config::{
    host_activity::{Publication, SourceFactory},
    retirement::Pool,
};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use tokio::sync::{oneshot, watch};
mod worker;

/// Keep this lease for the observing workflow's lifetime. Returned views own no
/// native authority. Last-consumer release requests stop; actual native evidence
/// and thread join, never reference counts, govern source capacity reuse.
/// Read guards stay in their local read scope. Release requests stop without
/// waiting for consumers to return borrowed publication guards.
pub trait ObservationLease: Send + Sync {
    fn updates(&self) -> watch::Receiver<Publication>;
}

struct Lease(Arc<Root>);
impl ObservationLease for Lease {
    fn updates(&self) -> watch::Receiver<Publication> {
        self.0.worker.updates()
    }
}
struct Root {
    worker: worker::Worker,
}

struct Registry {
    factory: Arc<dyn SourceFactory>,
    settings: Settings,
    pool: Pool,
    current: Mutex<Weak<Root>>,
}
impl Registry {
    fn new(factory: Arc<dyn SourceFactory>, settings: Settings) -> Result<Arc<Self>> {
        settings.validate()?;
        Ok(Arc::new(Self {
            factory,
            settings,
            pool: Pool::new(1)?,
            current: Mutex::new(Weak::new()),
        }))
    }
    fn open(&self) -> Result<Arc<Root>> {
        let mut current = self
            .current
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(root) = current.upgrade() {
            let value = *root.worker.updates().borrow();
            anyhow::ensure!(
                !matches!(value, Publication::Failed | Publication::Stopped),
                "Original machine-activity source is unavailable"
            );
            return Ok(root);
        }
        let (mut worker, response) =
            worker::start(&self.pool, self.factory.clone(), self.settings)?;
        let source = response
            .recv()
            .context("Machine-activity worker exited before startup")??;
        worker.attach(source);
        let root = Arc::new(Root { worker });
        *current = Arc::downgrade(&root);
        Ok(root)
    }
}

/// One admitted blocking constructor and one bounded result. Dropping an opening
/// retains the original creator through actual completion; rejected late results
/// drop their own root and sponsor original thread/backend cleanup.
struct Opening {
    creator: Task,
    response: oneshot::Receiver<Arc<Root>>,
}
impl Opening {
    fn start(create: impl FnOnce() -> Result<Arc<Root>> + Send + 'static) -> Result<Self> {
        let (reply, response) = oneshot::channel();
        let creator = Task::spawn_blocking(move || {
            let root = create()?;
            let _ = reply.send(root);
            Ok(())
        })?;
        Ok(Self { creator, response })
    }
    async fn wait(self) -> Result<Box<dyn ObservationLease>> {
        self.creator.await??;
        let root = self
            .response
            .await
            .context("Original machine-activity startup was withdrawn")?;
        Ok(Box::new(Lease(root)))
    }
}

fn shared() -> Result<Arc<Registry>> {
    static REGISTRY: OnceLock<Result<Arc<Registry>, String>> = OnceLock::new();
    cached(&REGISTRY, || {
        Registry::new(Arc::new(crate::activity::PowerFactory), Settings::default())
    })
}
fn cached(
    cell: &OnceLock<Result<Arc<Registry>, String>>,
    initialize: impl FnOnce() -> Result<Arc<Registry>>,
) -> Result<Arc<Registry>> {
    cell.get_or_init(|| initialize().map_err(|error| error.to_string()))
        .as_ref()
        .cloned()
        .map_err(|error| anyhow::anyhow!("{error}"))
}

/// Acquire shared Linux machine-activity observations, independently of media grants.
pub async fn acquire() -> Result<Box<dyn ObservationLease>> {
    Opening::start(|| shared()?.open())?.wait().await
}

#[cfg(test)]
mod tests;
