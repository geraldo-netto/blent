//! Input-device contracts; protocol dispatch does not depend on Linux event layout.
use super::InputConfig;
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::watch;

pub struct PenSample {
    pub position: (f64, f64, f64),
    pub tilt: (f64, f64),
    pub eraser: bool,
    pub action: u8,
    pub button: Option<bool>,
}

pub type DirectReply = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>;

pub trait InputSink: Send + Sync {
    fn release_all(&self);
    fn touch(&self, position: (f64, f64, f64), action: u8, slot: u8);
    fn pen(&self, sample: PenSample, enabled: bool);
    fn direct(&self, _: super::direct::Command) -> anyhow::Result<DirectReply> {
        anyhow::bail!("Direct input unavailable")
    }
    fn direct_status(&self) -> Option<super::direct::Status> {
        None
    }
}

impl<T: InputSink + ?Sized> InputSink for Arc<T> {
    fn release_all(&self) {
        self.as_ref().release_all();
    }
    fn touch(&self, position: (f64, f64, f64), action: u8, slot: u8) {
        self.as_ref().touch(position, action, slot);
    }
    fn pen(&self, sample: PenSample, enabled: bool) {
        self.as_ref().pen(sample, enabled);
    }
    fn direct(&self, command: super::direct::Command) -> anyhow::Result<DirectReply> {
        self.as_ref().direct(command)
    }
    fn direct_status(&self) -> Option<super::direct::Status> {
        self.as_ref().direct_status()
    }
}

pub trait InputBackend: Send + Sync {
    fn sink(&self) -> Arc<dyn InputSink>;
    /// Original native work that can outlive cancellation of the input server.
    fn native_completion(&self) -> Option<blent_config::retirement::Completion> {
        None
    }
    fn follow(
        &self,
        tablet: watch::Receiver<bool>,
        mode: watch::Receiver<bool>,
        config: InputConfig,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}
