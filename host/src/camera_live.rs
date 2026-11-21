//! One bounded latest-value mailbox per original camera session. Old sessions
//! retain their own settings; no revision can address a replacement or borrow a frame.
use anyhow::{ensure, Result};
use blent_config::camera::{
    adjustments::{Image, Status},
    tuning::{Capabilities, Command, Encoding, Sensor},
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
    sensor: Sensor,
    encoding: Encoding,
    capabilities: Option<Capabilities>,
    received: u64,
    receipt_status: u8,
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
                sensor: profile.sensor,
                encoding: Encoding::from_profile(&profile),
                capabilities: None,
                received: 0,
                receipt_status: 0,
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
        baseline.sensor = profile.sensor;
        let encoding = Encoding::from_profile(profile);
        encoding.write_profile(&mut baseline);
        let mut state = self.state.lock().unwrap();
        ensure!(!state.closed, "Camera session is stopping or stopped");
        // A different selected tablet must never alter the original tablet.
        ensure!(
            profile.target()? == self.profile.target()?,
            "Stop camera before changing tablet"
        );
        if let Some(capabilities) = state.capabilities {
            if let Err(error) = capabilities.accepts(profile.sensor) {
                let mut status = state.status();
                status.error = Some(error.to_string());
                return Ok(status);
            }
        }
        if (image, profile.sensor, encoding) != (state.image, state.sensor, state.encoding) {
            let revision = state
                .revision
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("Camera update revisions exhausted"))?;
            ensure!(
                revision <= i64::MAX as u64,
                "Camera update revisions exhausted"
            );
            state.image = image;
            state.sensor = profile.sensor;
            state.encoding = encoding;
            state.revision = revision;
        }
        state.restart = baseline != *profile;
        Ok(state.status())
    }
    pub fn capabilities(&self, capabilities: Capabilities) {
        let mut state = self.state.lock().unwrap();
        state.capabilities = Some(capabilities);
        state.received = 0;
        state.receipt_status = 0;
    }
    pub fn command(&self) -> Result<Command> {
        let state = self.state.lock().unwrap();
        ensure!(!state.closed, "Camera session is stopping or stopped");
        Ok(Command {
            revision: state.revision,
            sensor: state.sensor,
            encoding: state.encoding,
        })
    }
    pub fn receipt(&self, revision: u64, status: u8) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        ensure!(status <= 2, "Invalid camera control receipt");
        ensure!(
            revision <= state.revision,
            "Unknown camera control revision"
        );
        ensure!(
            revision >= state.received,
            "Camera control receipt moved backwards"
        );
        ensure!(
            revision != 0 || status == 0,
            "Unaddressed camera control error"
        );
        state.received = revision;
        state.receipt_status = status;
        Ok(())
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
    fn error(&self) -> Option<String> {
        if self.received != self.revision {
            return None;
        }
        match self.receipt_status {
            1 => Some("Camera rejected an unsupported adjustment".into()),
            2 => Some("Camera could not apply all adjustments; restart camera to retry".into()),
            _ => None,
        }
    }
    fn status(&self) -> Status {
        Status {
            pending: self.applied != self.revision
                || (self.capabilities.is_some() && self.received != self.revision),
            restart_required: self.restart,
            error: self.error(),
            capabilities: self.capabilities,
        }
    }
}
#[cfg(test)]
#[path = "camera_live/tests.rs"]
mod tests;
