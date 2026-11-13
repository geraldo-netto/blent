//! Display route creation and cleanup have one retained, bounded native owner.
use super::*;
use blent_config::retirement::{Admission, Completion, Observe, Pool};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use tokio::sync::{mpsc, oneshot, watch};
mod worker;

struct Control {
    serial: Arc<Mutex<String>>,
    expected: [(u16, u16); 2],
    reservation: Mutex<Option<Box<dyn Send>>>,
    settled: AtomicBool,
    native: Option<Completion>,
}
struct Worker {
    requests: mpsc::Sender<oneshot::Sender<Result<(), String>>>,
    stop: watch::Sender<bool>,
    failure: watch::Receiver<Option<String>>,
    completion: Completion,
}

/// Preexisting routes remain external. Admitted endpoints are exclusive through
/// original creation/removal settlement (T1345), including unacknowledged effects.
/// Drop cuts off admission; the retirement pool sponsors original cleanup.
pub struct Routes {
    serial: Arc<Mutex<String>>,
    expected: [(u16, u16); 2],
    worker: Option<Worker>,
    native: Option<Completion>,
}
impl Routes {
    pub fn new(serial: &str, ports: (u16, u16)) -> Result<Self> {
        ensure!(valid_serial(serial), "Invalid USB serial");
        Self::for_transport(serial, ports)
    }
    pub fn for_transport(serial: &str, ports: (u16, u16)) -> Result<Self> {
        check_serial(serial)?;
        blent_config::slot_ports(ports.0, ports.1, 1)?;
        Ok(Self {
            serial: Arc::new(Mutex::new(serial.into())),
            expected: [(8890, ports.0), (8891, ports.1)],
            worker: None,
            native: None,
        })
    }
    /// Caller proves the physical identity. The worker independently checks a
    /// network alias before mutation; retarget never cancels an original command.
    pub(crate) fn retarget(&mut self, serial: &str) -> Result<()> {
        check_serial(serial)?;
        *self.serial.lock().unwrap() = serial.into();
        Ok(())
    }
    pub fn serial(&self) -> String {
        self.serial.lock().unwrap().clone()
    }

    /// Bind one original session before exposing any native route worker.
    pub(crate) fn retain_native(&mut self, proof: Completion) -> Result<()> {
        ensure!(
            self.worker.is_none() && self.native.is_none(),
            "Native route lifetime already bound"
        );
        self.native = Some(proof);
        Ok(())
    }

    pub async fn prepare<C: Commands + Clone + 'static>(&mut self, adb: &Adb<C>) -> Result<()> {
        if self
            .worker
            .as_ref()
            .is_some_and(|w| w.completion.is_complete())
        {
            self.worker = None;
        }
        if self.worker.is_none() {
            self.worker = Some(self.start(adb.clone())?);
        }
        let worker = self.worker.as_ref().unwrap();
        ensure!(!*worker.stop.borrow(), "Display routes are retiring");
        let (reply, received) = oneshot::channel();
        worker
            .requests
            .try_send(reply)
            .map_err(|_| anyhow::anyhow!("Display route worker unavailable"))?;
        received
            .await
            .context("Display route preparation cancelled; owner retained")?
            .map_err(anyhow::Error::msg)
    }
    fn start<C: Commands + 'static>(&self, adb: Adb<C>) -> Result<Worker> {
        let admission = reserve()?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let control = Arc::new(Control {
            serial: self.serial.clone(),
            expected: self.expected,
            reservation: Mutex::new(None),
            settled: AtomicBool::new(false),
            native: self.native.clone(),
        });
        let (requests, receiver) = mpsc::channel(1);
        let (stop, stopping) = watch::channel(false);
        let (failure, errors) = watch::channel(None);
        let owned = control.clone();
        let final_report = failure.clone();
        let handle = std::thread::Builder::new()
            .name("blent-display-routes".into())
            .spawn(move || {
                runtime.block_on(worker::run(adb, owned, receiver, stopping, failure))
            })?;
        let completion = admission.completion();
        admission.retain(Retiring {
            thread: blent_config::retirement_work::Thread::new(handle),
            control,
            _report: final_report,
        });
        Ok(Worker {
            requests,
            stop,
            failure: errors,
            completion,
        })
    }
    pub async fn retire<C: Commands>(&mut self, _: &Adb<C>) -> Result<()> {
        let Some(worker) = &self.worker else {
            return Ok(());
        };
        worker.stop.send_replace(true);
        let mut failure = worker.failure.clone();
        failure.borrow_and_update(); // A retry observes a fresh cleanup outcome.
        while !worker.completion.is_complete() {
            if failure.changed().await.is_err() {
                // The last sender belongs to the native retirement observer.
                // Pool admission publication follows destruction of that observer.
                while !worker.completion.is_complete() {
                    tokio::task::yield_now().await;
                }
                break;
            }
            if let Some(error) = failure.borrow_and_update().clone() {
                anyhow::bail!("{error}; original display routes remain owned");
            }
        }
        Ok(())
    }

    pub fn completion(&self) -> Option<Completion> {
        self.worker.as_ref().map(|worker| worker.completion.clone())
    }
}
impl Drop for Routes {
    fn drop(&mut self) {
        if let Some(worker) = &self.worker {
            worker.stop.send_replace(true);
        }
    }
}
fn check_serial(serial: &str) -> Result<()> {
    ensure!(
        !serial.is_empty() && serial.len() <= 1024 && !serial.chars().any(char::is_control),
        "Invalid ADB serial"
    );
    Ok(())
}
fn reserve() -> Result<Admission> {
    static POOL: OnceLock<std::io::Result<Pool>> = OnceLock::new();
    let pool = POOL
        .get_or_init(|| {
            Pool::new(blent_config::retirement_limits::configured(
                "BLENT_DISPLAY_ROUTE_LIMIT",
                16,
                4096,
            )?)
        })
        .as_ref()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(pool.reserve()?)
}
struct Retiring {
    _report: watch::Sender<Option<String>>,
    thread: blent_config::retirement_work::Thread,
    control: Arc<Control>,
}
impl Observe for Retiring {
    fn poll_complete(&mut self) -> bool {
        self.thread.poll_complete()
            && self.control.settled.load(Ordering::Acquire)
            && self
                .control
                .native
                .as_ref()
                .is_none_or(Completion::is_complete)
    }
}

#[cfg(test)]
mod retirement_tests;
