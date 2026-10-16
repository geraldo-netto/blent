//! Portable capture boundary: values, immutable publication, observation and
//! owned work. The session binds listeners before invoking a backend.
use crate::{
    media::{EncoderSettings, VideoPacket},
    media_storage::MediaBytes,
};
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::watch;

pub type Work = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'static>>;
pub type Signal<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

/// Retain each original native completion before exposure or another await.
/// Cloned handles also retain creator lifetime, including delayed creation after
/// cancellation. This grants no authority to acquire or restart media.
pub trait NativeRetirement: Send + Sync {
    fn retain(&self, original: blent_config::retirement::Completion);
}

pub trait ViewerActivity: Send + Sync {
    /// Monotonic connection epoch; a changed value invalidates idle evidence.
    fn epoch(&self) -> u64;
}
pub trait PacketOutput: Send + Sync {
    /// Publish without copying payloads. Rejection returns the same owned packet.
    /// The implementation enforces retention and keyframe recovery limits.
    fn send(&self, packet: VideoPacket) -> Result<usize, VideoPacket>;
    fn receiver_count(&self) -> usize;
    fn viewers(&self) -> Arc<dyn ViewerActivity>;
}
#[derive(Clone)]
pub struct VideoOutput(Arc<dyn PacketOutput>);
impl VideoOutput {
    pub fn new(output: impl PacketOutput + 'static) -> Self {
        Self(Arc::new(output))
    }
    pub fn send(&self, packet: VideoPacket) -> Result<usize, VideoPacket> {
        self.0.send(packet)
    }
    pub fn receiver_count(&self) -> usize {
        self.0.receiver_count()
    }
    pub fn viewers(&self) -> Arc<dyn ViewerActivity> {
        self.0.viewers()
    }
}

/// Initial codec headers can precede the first frame; storage stays immutable.
pub trait HeaderPublication: Send + Sync {
    fn publish(&self, headers: Option<MediaBytes>);
    fn current(&self) -> Option<MediaBytes>;
}
pub trait RefreshRequests: Send + Sync {
    /// Consume pending keyframe demand. Repeated requests may coalesce.
    fn take(&self) -> bool;
}

pub trait ProfileLease: Send + Sync {
    /// Proven device identity and transport, captured in one generation.
    fn identity(&self) -> Option<(String, &'static str)>;
    fn active(&self) -> bool;
    fn retired(&self) -> Signal<'_>;
}
pub trait ProfileSource: Send + Sync {
    fn profile(&self) -> Box<dyn ProfileLease>;
}

pub struct CaptureContext {
    pub retirement: Arc<dyn NativeRetirement>,
    pub video: VideoOutput,
    pub headers: Arc<dyn HeaderPublication>,
    pub refresh: Arc<dyn RefreshRequests>,
    pub settings: watch::Sender<EncoderSettings>,
    pub settings_rx: watch::Receiver<EncoderSettings>,
    pub display: watch::Receiver<bool>,
    pub stop: watch::Receiver<bool>,
    pub identity: Arc<dyn ProfileSource>,
}
/// The owner starts and observes every returned future. Backends must not detach
/// native work. Completion retains success/error; panics are observed by owner.
pub struct CaptureWorkers {
    pub capture: Work,
    pub auxiliary: Vec<Work>,
}
pub trait CaptureBackend: Send {
    fn start(self: Box<Self>, context: CaptureContext) -> CaptureWorkers;
}
