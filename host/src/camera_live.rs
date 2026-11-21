//! One bounded latest-value mailbox per original camera session. Old sessions
//! retain their own settings; no revision can address a replacement or borrow a frame.
use anyhow::{ensure, Result};
use blent_config::camera::{
    adjustments::{Image, Status},
    CameraProfile,
};
use std::sync::{Arc, Mutex};

pub(crate) struct Session {
    pub profile: CameraProfile,
    state: Mutex<State>,
}
struct State {
    closed: bool,
    image: Image,
    revision: u64,
    applied: u64,
    restart: bool,
}
impl Session {
    pub fn new(profile: CameraProfile) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                closed: false,
                image: Image::from_profile(&profile),
                revision: 1,
                applied: 0,
                restart: false,
            }),
            profile,
        })
    }
    pub fn update(&self, profile: &CameraProfile) -> Result<Status> {
        profile.validate()?;
        let image = Image::from_profile(profile);
        let mut baseline = self.profile.clone();
        image.write_profile(&mut baseline);
        let mut state = self.state.lock().unwrap();
        ensure!(!state.closed, "Camera session is stopping or stopped");
        // A different selected tablet must never alter the original tablet.
        ensure!(
            profile.target()? == self.profile.target()?,
            "Stop camera before changing tablet"
        );
        if image != state.image {
            let revision = state
                .revision
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("Camera update revisions exhausted"))?;
            state.image = image;
            state.revision = revision;
        }
        state.restart = baseline != *profile;
        Ok(state.status())
    }
    pub fn snapshot(&self) -> (u64, Image) {
        let state = self.state.lock().unwrap();
        (state.revision, state.image)
    }
    pub fn applied(&self, revision: u64) {
        let mut state = self.state.lock().unwrap();
        if revision <= state.revision {
            state.applied = state.applied.max(revision);
        }
    }
    #[cfg(test)]
    pub fn status(&self) -> Status {
        self.state.lock().unwrap().status()
    }
    pub fn close(&self) {
        self.state.lock().unwrap().closed = true;
    }
}
impl State {
    fn status(&self) -> Status {
        Status {
            pending: self.applied != self.revision,
            restart_required: self.restart,
            error: None,
        }
    }
}
#[cfg(test)]
mod tests;
