//! One original stdout publisher; revocation synchronizes every watch update.
use super::{fifo, DetectedMode};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::watch;

#[derive(Clone)]
struct Channels {
    mode: watch::Sender<Option<DetectedMode>>,
    stream: watch::Sender<Option<(u32, u32)>>,
    fifo: watch::Sender<Option<fifo::Identity>>,
}

pub(super) struct Owner {
    active: Arc<AtomicBool>,
    channels: Channels,
}

#[derive(Clone)]
pub(super) struct Publisher {
    active: Arc<AtomicBool>,
    channels: Channels,
}

pub(super) fn open(
    mode: watch::Sender<Option<DetectedMode>>,
    stream: watch::Sender<Option<(u32, u32)>>,
    fifo: watch::Sender<Option<fifo::Identity>>,
) -> (Owner, Publisher) {
    let channels = Channels { mode, stream, fifo };
    let active = Arc::new(AtomicBool::new(true));
    (
        Owner {
            active: active.clone(),
            channels: channels.clone(),
        },
        Publisher { active, channels },
    )
}

impl Drop for Owner {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
        // An update that already checked authority holds this same watch lock.
        // These empty transactions wait it out without changing values/versions.
        barrier(&self.channels.mode);
        barrier(&self.channels.stream);
        barrier(&self.channels.fifo);
    }
}

fn barrier<T>(sender: &watch::Sender<T>) {
    sender.send_if_modified(|_| false);
}

fn publish<T: Copy>(sender: &watch::Sender<Option<T>>, active: &AtomicBool, value: T) {
    sender.send_if_modified(|current| {
        if !active.load(Ordering::Acquire) {
            return false;
        }
        *current = Some(value);
        true
    });
}

impl Publisher {
    pub(super) fn active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub(super) fn mode(&self, value: DetectedMode) {
        publish(&self.channels.mode, &self.active, value);
    }

    pub(super) fn stream(&self, value: (u32, u32)) {
        publish(&self.channels.stream, &self.active, value);
    }

    pub(super) fn fifo(&self, value: fifo::Identity) {
        publish(&self.channels.fifo, &self.active, value);
    }
}
