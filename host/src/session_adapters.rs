//! Session implementations of the independent capture ports.
use crate::{attachment, capture_contract::*, video_queue::VideoSender};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

impl ViewerActivity for AtomicU64 {
    fn epoch(&self) -> u64 {
        self.load(Ordering::Acquire)
    }
}
impl PacketOutput for VideoSender {
    fn send(&self, packet: crate::media::VideoPacket) -> Result<usize, crate::media::VideoPacket> {
        VideoSender::send(self, packet).map_err(|error| error.0)
    }
    fn receiver_count(&self) -> usize {
        VideoSender::receiver_count(self)
    }
    fn viewers(&self) -> Arc<dyn ViewerActivity> {
        self.viewer_activity()
    }
}
impl From<VideoSender> for VideoOutput {
    fn from(sender: VideoSender) -> Self {
        Self::new(sender)
    }
}
impl ProfileSource for attachment::Attachment {
    fn profile(&self) -> Box<dyn ProfileLease> {
        Box::new(self.lease())
    }
    fn changed(&self) -> Signal<'static> {
        let mut original = self.generation_updates();
        Box::pin(async move {
            let _ = original.changed().await;
        })
    }
}
impl ProfileLease for attachment::Lease {
    fn identity(&self) -> Option<(String, &'static str)> {
        self.profile_identity()
    }
    fn active(&self) -> bool {
        self.apply(|| {})
    }
    fn retired(&self) -> Signal<'_> {
        let mut retirement = self.retirement();
        Box::pin(async move { retirement.retired().await })
    }
}

impl HeaderPublication for crate::media::CodecConfig {
    fn publish(&self, headers: Option<crate::media_storage::MediaBytes>) {
        crate::media::CodecConfig::publish(self, headers);
    }
    fn current(&self) -> Option<crate::media_storage::MediaBytes> {
        crate::media::CodecConfig::current(self)
    }
}
impl RefreshRequests for std::sync::atomic::AtomicBool {
    fn take(&self) -> bool {
        self.swap(false, Ordering::AcqRel)
    }
}
