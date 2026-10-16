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

/// Read-only original connection grant. A source owns its publication; closed
/// sources deny support. Observation conveys no capture or retirement authority.
pub trait LivenessCapability: Send + Sync {
    fn supported(&self) -> bool;
    /// Arm before inspecting supported(), so a concurrent change cannot be lost.
    /// The returned observation owns only a read view of this original source.
    fn changed(&self) -> Signal<'static>;
}

/// Immutable observation of continuous viewer membership, never a media grant.
#[derive(Clone)]
pub struct ViewerPresence {
    count: usize,
    continuity: Arc<()>,
}
impl ViewerPresence {
    pub fn new(count: usize, continuity: Arc<()>) -> Self {
        Self { count, continuity }
    }
    pub fn present(&self) -> bool {
        self.count != 0
    }
    pub fn continues(&self, original: &Self) -> bool {
        self.present() && original.present() && Arc::ptr_eq(&self.continuity, &original.continuity)
    }
}

pub trait ViewerActivity: Send + Sync {
    /// None explicitly denotes a composition without viewer-presence observation.
    fn presence(&self) -> Option<ViewerPresence> {
        None
    }

    /// Monotonic connection epoch; a changed value invalidates idle evidence.
    fn epoch(&self) -> u64;
    /// True only with viewers present and every original viewer acknowledging
    /// liveness. Unmanaged/legacy/closed sources deny peer-dependent pausing.
    fn liveness_ready(&self) -> bool {
        false
    }
    /// Arm before reading current state. Unsupported compositions have no wake.
    fn changed(&self) -> Signal<'static> {
        Box::pin(std::future::pending())
    }
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
    /// Arm before reading profile(), so replacement/loss cannot be missed.
    /// This notification provides no authority to acquire or retire resources.
    fn changed(&self) -> Signal<'static>;
}

pub struct CaptureContext {
    pub activity_preferences: blent_config::host_activity::Settings,
    /// Shared read-only machine observations. None explicitly means this
    /// composition supplies no supported activity source; never native authority.
    pub activity: Option<watch::Receiver<blent_config::host_activity::Publication>>,
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
