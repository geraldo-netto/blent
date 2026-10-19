//! T543 manual host ownership. Preferences never start capture by themselves.
use anyhow::Result;
use blent_config::camera::{CameraBackend, CameraPreview, CameraProfile, CameraState as State};
use std::{future::Future, sync::Arc};
use tokio::sync::watch;

#[derive(Clone)]
pub struct Report {
    state: watch::Sender<State>,
    preview: watch::Sender<Option<Arc<CameraPreview>>>,
    permit: Option<crate::report_scope::Permit>,
}
impl Report {
    pub fn new(
        state: watch::Sender<State>,
        preview: watch::Sender<Option<Arc<CameraPreview>>>,
    ) -> Self {
        Self {
            state,
            preview,
            permit: None,
        }
    }
    /// Native callbacks receive only their original attempt's publication permit.
    pub fn scoped(&self, permit: crate::report_scope::Permit) -> Self {
        Self {
            state: self.state.clone(),
            preview: self.preview.clone(),
            permit: Some(permit),
        }
    }
    fn publish(&self, action: impl FnOnce()) {
        match &self.permit {
            Some(permit) => permit.publish(action),
            None => action(),
        }
    }
    pub fn state(&self) -> State {
        self.state.borrow().clone()
    }
    pub fn preview(&self) -> Option<Arc<CameraPreview>> {
        self.preview.borrow().clone()
    }
    pub fn publish_preview(&self, preview: Option<Arc<CameraPreview>>) {
        self.publish(|| {
            self.preview.send_replace(preview);
        });
    }

    /// Owning workflow progress: Paused/Finished require actual native settlement.
    pub fn activity(&self, phase: blent_config::auxiliary_activity::Phase) {
        use blent_config::auxiliary_activity::Phase;
        self.update(match phase {
            Phase::Paused => State::Paused,
            Phase::Starting => State::Starting,
            Phase::Retiring => State::Stopping,
            Phase::Finished => State::Stopped,
        });
    }
    pub fn update(&self, state: State) {
        self.publish(|| {
            if state != State::Streaming {
                self.preview.send_replace(None);
            }
            self.state.send_replace(state);
        });
    }
}

pub struct Controller {
    command: Option<watch::Sender<Option<CameraProfile>>>,
    state: watch::Receiver<State>,
    preview: watch::Receiver<Option<Arc<CameraPreview>>>,
    worker: Option<crate::control_workers::Worker>,
}

impl Controller {
    pub fn new<F, Fut>(session: F) -> Result<Self>
    where
        F: Fn(CameraProfile, watch::Receiver<bool>, Report) -> Fut + Send + 'static,
        Fut: Future<Output = Result<()>>,
    {
        Self::with_pool(crate::control_workers::WorkerPool::shared()?, session)
    }

    pub fn with_pool<F, Fut>(pool: crate::control_workers::WorkerPool, session: F) -> Result<Self>
    where
        F: Fn(CameraProfile, watch::Receiver<bool>, Report) -> Fut + Send + 'static,
        Fut: Future<Output = Result<()>>,
    {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let (command, commands) = watch::channel(None);
        let (status, state) = watch::channel(State::Stopped);
        let (images, preview) = watch::channel(None);
        let report = Report::new(status, images);
        let worker = crate::control_workers::Worker::spawn(pool, "blent-camera", move || {
            crate::control_outcome::worker(
                || runtime.block_on(supervise(commands, report.clone(), session)),
                |error| report.update(State::Failed(format!("{error:#}"))),
            );
        })?;
        Ok(Self {
            command: Some(command),
            state,
            preview,
            worker: Some(worker),
        })
    }
    pub fn shutdown(mut self) -> crate::control_workers::Completion {
        let complete = self.worker.as_ref().unwrap().completion();
        self.retire();
        complete
    }
    fn retire(&mut self) {
        drop(self.command.take());
        drop(self.worker.take());
    }
}

impl CameraBackend for Controller {
    fn start(&self, options: CameraProfile) -> Result<()> {
        options.validate()?;
        self.command.as_ref().unwrap().send(Some(options))?;
        Ok(())
    }

    fn stop(&self) {
        let _ = self.command.as_ref().unwrap().send(None);
    }

    fn state(&self) -> State {
        self.state.borrow().clone()
    }
    fn preview(&self) -> Option<Arc<CameraPreview>> {
        self.preview.borrow().clone()
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.retire();
    }
}

async fn supervise<F, Fut>(
    mut commands: watch::Receiver<Option<CameraProfile>>,
    status: Report,
    session: F,
) where
    F: Fn(CameraProfile, watch::Receiver<bool>, Report) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    loop {
        let options = commands.borrow_and_update().clone();
        if let Some(options) = options {
            if !manage_session(options, &mut commands, &status, &session).await {
                break;
            }
        } else {
            status.update(State::Stopped);
            if commands.changed().await.is_err() {
                break;
            }
        }
    }
    status.update(State::Stopped);
}

async fn manage_session<F, Fut>(
    options: CameraProfile,
    commands: &mut watch::Receiver<Option<CameraProfile>>,
    status: &Report,
    session: &F,
) -> bool
where
    F: Fn(CameraProfile, watch::Receiver<bool>, Report) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let (stop, stopped) = watch::channel(false);
    status.update(State::Starting);
    let work = crate::control_outcome::run(|| session(options, stopped, status.clone()));
    tokio::pin!(work);
    tokio::select! {
        result = &mut work => {
            status.update(match result {
                Ok(()) => State::Stopped,
                Err(error) => State::Failed(format!("{error:#}")),
            });
            // Failure never restarts a lens without another desktop action.
            commands.changed().await.is_ok()
        }
        changed = commands.changed() => {
            status.update(State::Stopping);
            stop.send_replace(true);
            // Completion still owns final frames, child retirement and ADB cleanup.
            if let Err(error) = work.await {
                status.update(State::Failed(format!("{error:#}")));
                return changed.is_ok() && commands.changed().await.is_ok();
            }
            changed.is_ok()
        }
    }
}

/// Native adapters use this bounded ownership signal to begin graceful cleanup.
pub async fn cancelled(stop: &mut watch::Receiver<bool>) {
    if !*stop.borrow_and_update() {
        let _ = stop.changed().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[tokio::test]
    async fn t543_manual_start_switch_stop_and_owner_exit_retire_in_order() {
        let (command, commands) = watch::channel(None);
        let (status, mut state) = watch::channel(State::Stopped);
        let status = Report::new(status, watch::channel(None).0);
        let events = Arc::new(Mutex::new(Vec::new()));
        let recording = events.clone();
        let owner = tokio::spawn(supervise(
            commands,
            status,
            move |profile, mut stop, status| {
                let events = recording.clone();
                async move {
                    events
                        .lock()
                        .unwrap()
                        .push(format!("start {:?}", profile.lens));
                    status.update(State::Streaming);
                    cancelled(&mut stop).await;
                    events
                        .lock()
                        .unwrap()
                        .push(format!("stop {:?}", profile.lens));
                    Ok(())
                }
            },
        ));
        tokio::task::yield_now().await;
        assert!(
            events.lock().unwrap().is_empty(),
            "T543 preferences must not start capture"
        );
        command.send(Some(CameraProfile::default())).unwrap();
        state.wait_for(|s| *s == State::Streaming).await.unwrap();
        command
            .send(Some(CameraProfile {
                lens: blent_config::camera::Lens::Rear,
                ..Default::default()
            }))
            .unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        command.send(None).unwrap();
        state.wait_for(|s| *s == State::Stopped).await.unwrap();
        assert_eq!(
            *events.lock().unwrap(),
            ["start Front", "stop Front", "start Rear", "stop Rear"]
        );
        command.send(Some(CameraProfile::default())).unwrap();
        state.wait_for(|s| *s == State::Streaming).await.unwrap();
        drop(command);
        owner.await.unwrap();
        assert_eq!(events.lock().unwrap().last().unwrap(), "stop Front");
    }

    #[tokio::test]
    async fn t543_failed_session_waits_for_explicit_retry() {
        let (command, commands) = watch::channel(None);
        let (status, mut state) = watch::channel(State::Stopped);
        let status = Report::new(status, watch::channel(None).0);
        let owner = tokio::spawn(supervise(commands, status, |_, _, _| async {
            anyhow::bail!("missing webcam")
        }));
        command.send(Some(CameraProfile::default())).unwrap();
        state
            .wait_for(|s| matches!(s, State::Failed(_)))
            .await
            .unwrap();
        tokio::task::yield_now().await;
        assert_eq!(*state.borrow(), State::Failed("missing webcam".into()));
        command.send(None).unwrap();
        state.wait_for(|s| *s == State::Stopped).await.unwrap();
        drop(command);
        owner.await.unwrap();
        let (_sender, mut already_stopped) = watch::channel(true);
        cancelled(&mut already_stopped).await;
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
                report.update(State::Streaming);
                entered.send(()).unwrap();
                crate::camera_control::cancelled(&mut stop).await;
                held.lock().unwrap().recv().unwrap();
                Ok(())
            }
        })
        .unwrap();
        controller.start(CameraProfile::default()).unwrap();
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
    fn t543_controller_owns_worker_and_validates_before_start() {
        let owner = Controller::new(|_, _, _| async { anyhow::bail!("missing webcam") }).unwrap();
        assert_eq!(owner.state(), State::Stopped);
        assert!(owner.preview().is_none());
        assert!(owner
            .start(CameraProfile {
                fps: 0,
                ..Default::default()
            })
            .is_err());
        owner
            .start(CameraProfile {
                ..Default::default()
            })
            .unwrap();
        for _ in 0..100 {
            if matches!(owner.state(), State::Failed(_)) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(matches!(owner.state(), State::Failed(_)));
        owner.stop();
    }
}
