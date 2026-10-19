//! Manual audio ownership: no saved setting restores a capture session.
use anyhow::Result;
use blent_config::audio::{AudioController, AudioOptions, AudioState, AudioStatus};
use std::{
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::sync::watch;

#[derive(Clone)]
pub struct Report {
    status: watch::Sender<AudioStatus>,
    retirement: Arc<Mutex<retirement::Tracker>>,
    permit: Option<crate::report_scope::Permit>,
}
mod retirement;
impl Report {
    pub fn new(status: watch::Sender<AudioStatus>) -> Self {
        Self {
            status,
            retirement: Arc::new(Mutex::new(Default::default())),
            permit: None,
        }
    }
    pub fn scoped(&self, permit: crate::report_scope::Permit) -> Self {
        Self {
            status: self.status.clone(),
            retirement: self.retirement.clone(),
            permit: Some(permit),
        }
    }
    fn publish(&self, action: impl FnOnce()) {
        match &self.permit {
            Some(permit) => permit.publish(action),
            None => action(),
        }
    }
    /// Record proof immediately after native creation, before publishing it or
    /// returning from the owning backend. Observation does not own the resource.
    pub fn retain(&self, completion: blent_config::retirement::Completion) -> Result<()> {
        self.retirement
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(completion)
    }
    pub fn status(&self) -> AudioStatus {
        self.status.borrow().clone()
    }
    pub(crate) fn native_retired(&self) -> bool {
        self.retirement
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .complete()
    }
    pub(crate) async fn wait_native(&self) {
        if self.native_retired() {
            return;
        }
        self.update(
            AudioState::Stopping,
            "Waiting for actual audio resource cleanup…",
        );
        while !self.native_retired() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    /// Workflow progress is independent of native streaming reports. Paused is
    /// published only after the previous attempt and native callbacks settle.
    pub fn activity(&self, phase: blent_config::auxiliary_activity::Phase) {
        use blent_config::auxiliary_activity::Phase;
        let (state, detail) = match phase {
            Phase::Paused => (
                AudioState::Paused,
                "Waiting for host availability. Stop cancels pending start or resume.",
            ),
            Phase::Starting => (AudioState::Starting, "Starting audio…"),
            Phase::Retiring => (AudioState::Stopping, "Waiting for audio resource cleanup…"),
            Phase::Finished => (AudioState::Stopped, "Stopped"),
        };
        self.update(state, detail);
    }
    pub fn update(&self, state: AudioState, detail: impl Into<String>) {
        self.publish(|| {
            self.status.send_replace(AudioStatus {
                state,
                detail: detail.into(),
            });
        });
    }
}

pub struct Controller {
    commands: Option<watch::Sender<Option<AudioOptions>>>,
    status: watch::Receiver<AudioStatus>,
    worker: Option<crate::control_workers::Worker>,
    admitted: Arc<Mutex<bool>>,
}
impl Controller {
    pub fn new<F, Fut>(run: F) -> Result<Self>
    where
        F: Fn(AudioOptions, watch::Receiver<bool>, Report) -> Fut + Send + 'static,
        Fut: Future<Output = Result<()>>,
    {
        Self::with_pool(crate::control_workers::WorkerPool::shared()?, run)
    }

    pub fn with_pool<F, Fut>(pool: crate::control_workers::WorkerPool, run: F) -> Result<Self>
    where
        F: Fn(AudioOptions, watch::Receiver<bool>, Report) -> Fut + Send + 'static,
        Fut: Future<Output = Result<()>>,
    {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let (commands, receiver) = watch::channel(None);
        let (sender, status) = watch::channel(AudioStatus::default());
        let admitted = Arc::new(Mutex::new(false));
        let worker_admission = admitted.clone();
        let worker =
            crate::control_workers::Worker::spawn(pool, "blent-audio-control", move || {
                let report = Report::new(sender);
                crate::control_outcome::worker(
                    || {
                        runtime.block_on(supervise(
                            receiver,
                            report.clone(),
                            run,
                            worker_admission.clone(),
                        ))
                    },
                    |error| {
                        runtime.block_on(report.wait_native());
                        complete(&report, &worker_admission, Err(error));
                    },
                );
            })?;
        Ok(Self {
            commands: Some(commands),
            status,
            worker: Some(worker),
            admitted,
        })
    }
    pub fn shutdown(mut self) -> crate::control_workers::Completion {
        let complete = self.worker.as_ref().unwrap().completion();
        self.retire();
        complete
    }
    fn retire(&mut self) {
        drop(self.commands.take());
        drop(self.worker.take());
    }
}
impl AudioController for Controller {
    fn start(&self, options: AudioOptions) -> Result<()> {
        options.validate()?;
        let mut admitted = self.admitted.lock().unwrap();
        anyhow::ensure!(!*admitted, "Stop audio before changing an active session");
        self.commands.as_ref().unwrap().send(Some(options))?;
        *admitted = true;
        Ok(())
    }
    fn stop(&self) {
        let _ = self.commands.as_ref().unwrap().send(None);
    }
    fn status(&self) -> AudioStatus {
        self.status.borrow().clone()
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        self.retire();
    }
}

async fn supervise<F, Fut>(
    mut commands: watch::Receiver<Option<AudioOptions>>,
    report: Report,
    run: F,
    admitted: Arc<Mutex<bool>>,
) where
    F: Fn(AudioOptions, watch::Receiver<bool>, Report) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    loop {
        let options = commands.borrow_and_update().clone();
        if let Some(options) = options {
            if !active(options, &mut commands, &report, &run, &admitted).await {
                break;
            }
        } else {
            {
                let mut reservation = admitted.lock().unwrap();
                // Start may have arrived after the earlier empty snapshot.
                if commands.borrow().is_none() {
                    report.update(AudioState::Stopped, "Stopped");
                    *reservation = false;
                }
            }
            if commands.changed().await.is_err() {
                break;
            }
        }
    }
}

async fn active<F, Fut>(
    options: AudioOptions,
    commands: &mut watch::Receiver<Option<AudioOptions>>,
    report: &Report,
    run: &F,
    admitted: &Mutex<bool>,
) -> bool
where
    F: Fn(AudioOptions, watch::Receiver<bool>, Report) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let (stop, stopped) = watch::channel(false);
    let detail = match options.profile.direction {
        blent_config::audio::Direction::Microphone => {
            "Open Blent on the tablet and allow microphone access."
        }
        blent_config::audio::Direction::Speakers => {
            "Open Blent on the tablet for speaker playback."
        }
    };
    report.update(AudioState::Starting, detail);
    let work = crate::control_outcome::run(|| run(options, stopped, report.clone()));
    tokio::pin!(work);
    tokio::select! {
        result = &mut work => {
            report.wait_native().await;
            complete(report, admitted, result);
            commands.changed().await.is_ok()
        }
        changed = commands.changed() => {
            report.update(AudioState::Stopping, "Stopping audio…");
            stop.send_replace(true);
            let result = work.await;
            report.wait_native().await;
            if let Err(error) = result {
                complete(report, admitted, Err(error));
                return changed.is_ok() && commands.changed().await.is_ok();
            }
            changed.is_ok()
        }
    }
}

fn complete(report: &Report, admitted: &Mutex<bool>, result: Result<()>) {
    let mut reservation = admitted
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match result {
        Ok(()) => report.update(AudioState::Stopped, "Stopped"),
        Err(error) => report.update(AudioState::Failed, format!("{error:#}")),
    }
    *reservation = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    use blent_config::audio::Direction;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::{Duration, Instant},
    };

    #[test]
    fn t977_invalid_target_is_rejected_before_control_admission() {
        let (sender, receiver) = watch::channel(None);
        let (_status, status) = watch::channel(AudioStatus::default());
        let controller = Controller {
            commands: Some(sender),
            status,
            worker: None,
            admitted: Arc::new(Mutex::new(false)),
        };
        let mut options = AudioOptions::new(Direction::Speakers);
        options.usb_peer = Some("a".repeat(64));
        options.serial = Some("other-tablet".into());
        assert!(
            controller.start(options).is_err(),
            "T977 ambiguous target admitted"
        );
        assert!(receiver.borrow().is_none());
        assert!(!*controller.admitted.lock().unwrap());
        assert_eq!(controller.status().state, AudioState::Stopped);
    }

    #[test]
    fn t811_drop_returns_while_native_retirement_is_held() {
        let (entered, ready) = std::sync::mpsc::channel();
        let (release, held) = std::sync::mpsc::channel();
        let held = Arc::new(std::sync::Mutex::new(held));
        let pool = crate::control_workers::WorkerPool::new(1).unwrap();
        let controller = Controller::with_pool(pool.clone(), move |_, mut stop, report| {
            let entered = entered.clone();
            let held = held.clone();
            async move {
                report.update(AudioState::Streaming, "ready");
                entered.send(()).unwrap();
                crate::camera_control::cancelled(&mut stop).await;
                held.lock().unwrap().recv().unwrap();
                Ok(())
            }
        })
        .unwrap();
        controller
            .start(AudioOptions::new(Direction::Microphone))
            .unwrap();
        ready
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        let complete = controller.worker.as_ref().unwrap().completion();
        let (dropped, done) = std::sync::mpsc::channel();
        let caller = std::thread::spawn(move || {
            drop(controller);
            dropped.send(()).unwrap();
        });
        let responsive = done
            .recv_timeout(std::time::Duration::from_millis(200))
            .is_ok();
        assert!(Controller::with_pool(pool.clone(), |_, _, _| async { Ok(()) }).is_err());
        release.send(()).unwrap();
        caller.join().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !complete.is_complete() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let next = Controller::with_pool(pool, |_, _, _| async { Ok(()) }).unwrap();
        let retired = next.shutdown();
        while !retired.is_complete() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(responsive, "T811 destructor waits for held native cleanup");
    }

    #[test]
    fn t718_pending_start_reserves_admission_before_worker_status() {
        let (sender, receiver) = watch::channel(None);
        let (_status, status) = watch::channel(AudioStatus::default());
        let controller = Controller {
            commands: Some(sender),
            status,
            worker: None,
            admitted: Arc::new(Mutex::new(false)),
        };
        let options = AudioOptions::new(Direction::Microphone);
        controller.start(options.clone()).unwrap();
        assert!(receiver.borrow().is_some());
        assert!(
            controller.start(options.clone()).is_err(),
            "T718 second Start was admitted before worker published Starting"
        );
        controller.stop();
        assert!(
            controller.start(options).is_err(),
            "T718 Start was admitted before Stop retirement was acknowledged"
        );
    }
    fn wait(controller: &Controller, state: AudioState) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while controller.status().state != state {
            assert!(Instant::now() < deadline, "{:?}", controller.status());
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn t718_explicit_start_stop_failure_and_drop_retire() {
        let retired = Arc::new(AtomicUsize::new(0));
        let count = retired.clone();
        let controller = Controller::new(move |options, mut stop, report| {
            let count = count.clone();
            async move {
                if options.serial.as_deref() == Some("fail") {
                    anyhow::bail!("unavailable");
                }
                report.update(AudioState::Streaming, "ready");
                crate::camera_control::cancelled(&mut stop).await;
                count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .unwrap();
        let options = AudioOptions::new(Direction::Microphone);
        let mut bad = options.clone();
        bad.profile.buffer_ms = 1;
        assert!(controller.start(bad).is_err());
        assert_eq!(controller.status().state, AudioState::Stopped);
        controller.start(options.clone()).unwrap();
        wait(&controller, AudioState::Streaming);
        assert!(controller.start(options.clone()).is_err());
        controller.stop();
        wait(&controller, AudioState::Stopped);
        assert_eq!(retired.load(Ordering::SeqCst), 1);
        let mut failure = options.clone();
        failure.serial = Some("fail".into());
        controller.start(failure).unwrap();
        wait(&controller, AudioState::Failed);
        assert!(controller.status().detail.contains("unavailable"));
        controller.start(options).unwrap();
        wait(&controller, AudioState::Streaming);
        let complete = controller.shutdown();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !complete.is_complete() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(retired.load(Ordering::SeqCst), 2);
        drop(Controller::new(|_, _, _| async { Ok(()) }).unwrap());
    }
}
