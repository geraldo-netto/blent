//! Published monitor snapshots. Redraw never calls a native inventory provider.
use blent_config::{
    input_mapping::{Monitor, MonitorInventory, Snapshot},
    retirement::Pool,
    retirement_work::Thread,
};
use std::{
    sync::{mpsc, Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

#[derive(Clone, Default)]
pub struct Published(Arc<Mutex<Reading>>);
#[derive(Clone, Default)]
pub struct Reading {
    monitors: Arc<[Monitor]>,
    collected: Option<Instant>,
    error: Option<String>,
}
impl Reading {
    pub fn monitors(&self) -> &[Monitor] {
        &self.monitors
    }
    pub fn notice(&self, now: Instant) -> Option<&str> {
        if let Some(error) = self.error.as_deref() {
            return Some(error);
        }
        match self.collected {
            None => Some("Monitor inventory unavailable: waiting for discovery"),
            Some(when) if now.saturating_duration_since(when) >= Duration::from_secs(6) => {
                Some("Monitor inventory stale: discovery has not completed")
            }
            Some(_) => None,
        }
    }
}
impl Published {
    pub fn read(&self) -> Reading {
        self.0.lock().unwrap().clone()
    }
    pub(super) fn publish(&self, result: Result<Snapshot, String>, now: Instant) {
        let mut reading = self.0.lock().unwrap();
        match result {
            Ok(snapshot) => {
                reading.monitors = snapshot.monitors().into();
                reading.collected = Some(now);
                reading.error = None;
            }
            Err(error) => reading.error = Some(format!("Monitor inventory unavailable: {error}")),
        }
    }
}
pub fn discover(
    source: impl MonitorInventory + Send + 'static,
    repaint: impl Fn() + Send + 'static,
) -> (Published, Option<Worker>) {
    let published = Published::default();
    let worker = Worker::start(published.clone(), source, repaint)
        .map_err(|error| published.publish(Err(error.to_string()), Instant::now()))
        .ok();
    (published, worker)
}
pub struct Worker {
    _stop: mpsc::Sender<()>,
}
impl Worker {
    pub fn start(
        published: Published,
        source: impl MonitorInventory + Send + 'static,
        repaint: impl Fn() + Send + 'static,
    ) -> std::io::Result<Self> {
        static POOL: OnceLock<std::io::Result<Pool>> = OnceLock::new();
        let pool = POOL
            .get_or_init(|| {
                Pool::new(blent_config::retirement_limits::configured(
                    "BLENT_MONITOR_WORKERS",
                    1,
                    64,
                )?)
            })
            .as_ref()
            .map_err(|e| std::io::Error::new(e.kind(), e.to_string()))?;
        Self::with_pool(pool, published, source, repaint, Duration::from_secs(2))
    }
    fn with_pool(
        pool: &Pool,
        published: Published,
        source: impl MonitorInventory + Send + 'static,
        repaint: impl Fn() + Send + 'static,
        interval: Duration,
    ) -> std::io::Result<Self> {
        let admission = pool.reserve()?;
        let (stop, stopped) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("blent-monitor-inventory".into())
            .spawn(move || {
                while !matches!(stopped.try_recv(), Err(mpsc::TryRecvError::Disconnected)) {
                    let result = source.snapshot().map_err(|e| e.to_string());
                    if matches!(stopped.try_recv(), Err(mpsc::TryRecvError::Disconnected)) {
                        break;
                    }
                    published.publish(result, Instant::now());
                    repaint();
                    if !matches!(
                        stopped.recv_timeout(interval),
                        Err(mpsc::RecvTimeoutError::Timeout)
                    ) {
                        break;
                    }
                }
            })?;
        admission.retain(Thread::new(handle));
        Ok(Self { _stop: stop })
    }
}
#[cfg(test)]
mod tests;
