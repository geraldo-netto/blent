//! One linear probe creator and its original controller/native retirement proof.
use anyhow::{Context, Result};
use blent::{
    camera_control::{Controller, Report},
    control_workers::{Completion, WorkerPool},
};
use blent_config::camera::{CameraBackend, CameraProfile};
use std::{
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::watch;

pub(super) struct Creation(watch::Sender<Option<Completion>>);
pub(super) struct Retirement(watch::Receiver<Option<Completion>>);
pub(super) struct Owner {
    controller: Option<Controller>,
    creation: Creation,
}

pub(super) fn channel() -> (Creation, Retirement) {
    let (created, retired) = watch::channel(None);
    (Creation(created), Retirement(retired))
}

impl Creation {
    /// Bind one already-admitted native owner and seal this unique creator.
    pub(super) fn bind(self, original: Completion) {
        self.0.send_replace(Some(original));
    }

    pub(super) fn start<F, Fut>(self, profile: CameraProfile, session: F) -> Result<Owner>
    where
        F: FnOnce(CameraProfile, watch::Receiver<bool>, Report) -> Fut + Send + 'static,
        Fut: Future<Output = Result<()>>,
    {
        profile.validate()?;
        let session = Arc::new(Mutex::new(Some(session)));
        let controller =
            Controller::with_pool(WorkerPool::new(1)?, move |profile, stop, report| {
                let session = session
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .take();
                async move {
                    let session = session.context("Probe session already started")?;
                    session(profile, stop, report).await
                }
            })?;
        let owner = Owner {
            controller: Some(controller),
            creation: self,
        };
        owner.controller.as_ref().unwrap().start(profile)?;
        Ok(owner)
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        if let Some(controller) = self.controller.take() {
            self.creation.0.send_replace(Some(controller.shutdown()));
        }
    }
}

impl Retirement {
    pub(super) async fn wait(mut self) {
        // Closing the unique creator seals whether an original controller exists.
        // A sealed controller proof also covers any funded late native creation.
        while self.0.changed().await.is_ok() {}
        let original = self.0.borrow().clone();
        if let Some(original) = original {
            while !original.is_complete() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
}
