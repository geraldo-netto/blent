//! T820: independent capture ports preserve ownership and terminal outcomes.
use blent::{
    capture_contract::*,
    media::{EncoderGeneration, VideoPacket},
    media_storage::MediaBytes,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

struct Viewers(Arc<AtomicU64>);
impl ViewerActivity for Viewers {
    fn epoch(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }
}
struct FakeOutput {
    retained: Arc<Mutex<Option<VideoPacket>>>,
    viewers: Arc<AtomicU64>,
}
impl PacketOutput for FakeOutput {
    fn send(&self, packet: VideoPacket) -> Result<usize, VideoPacket> {
        if packet.data.is_empty() {
            return Err(packet);
        }
        *self.retained.lock().unwrap() = Some(packet);
        Ok(1)
    }
    fn receiver_count(&self) -> usize {
        1
    }
    fn viewers(&self) -> Arc<dyn ViewerActivity> {
        Arc::new(Viewers(self.viewers.clone()))
    }
}
fn packet(data: MediaBytes, generation: &EncoderGeneration) -> VideoPacket {
    VideoPacket {
        data,
        is_idr: true,
        seq: 1,
        codec_config: None,
        generation: generation.active.clone(),
    }
}
#[test]
fn t820_fake_output_preserves_immutable_backing_and_generation() {
    let retained = Arc::new(Mutex::new(None));
    let epoch = Arc::new(AtomicU64::new(1));
    let output = VideoOutput::new(FakeOutput {
        retained: retained.clone(),
        viewers: epoch.clone(),
    });
    let generation = EncoderGeneration::new();
    let data = MediaBytes::from(vec![1, 2, 3, 4]);
    let address = data.as_ptr();
    assert_eq!(output.receiver_count(), 1);
    assert!(output.clone().send(packet(data, &generation)).is_ok());
    assert_eq!(
        retained.lock().unwrap().as_ref().unwrap().data.as_ptr(),
        address,
        "T820: boundary copied frame bytes"
    );
    let rejected = output
        .send(packet(MediaBytes::from_static(b""), &generation))
        .err()
        .unwrap();
    assert!(rejected.data.is_empty());
    assert!(Arc::ptr_eq(&rejected.generation, &generation.active));
    let observer = output.viewers();
    assert_eq!(observer.epoch(), 1);
    epoch.store(2, Ordering::Release);
    assert_eq!(observer.epoch(), 2);
    drop(generation);
    assert!(!retained
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .generation
        .load(Ordering::Acquire));
}
#[tokio::test]
async fn t820_real_queue_and_attachment_adapters_obey_capture_contracts() {
    let (queue, receiver) = blent::video_queue::channel(2, Default::default());
    let output: VideoOutput = queue.clone().into();
    let observer = output.viewers();
    assert_eq!(observer.epoch(), 0);
    let mut subscriber = queue.subscribe();
    assert_eq!(observer.epoch(), 1);
    assert_eq!(output.receiver_count(), 2);
    let generation = EncoderGeneration::new();
    let data = MediaBytes::from(vec![4, 3, 2, 1]);
    let address = data.as_ptr();
    assert!(output.send(packet(data, &generation)).is_ok());
    assert_eq!(subscriber.recv().await.unwrap().data.as_ptr(), address);
    drop((subscriber, receiver));
    let data = MediaBytes::from_static(b"rejected");
    let address = data.as_ptr();
    assert_eq!(
        output
            .send(packet(data, &generation))
            .err()
            .unwrap()
            .data
            .as_ptr(),
        address
    );
    let settings = blent::media::EncoderSettings {
        encoder: "libx264".into(),
        fps: 30,
        bitrate: 20000,
        width: 640,
        height: 480,
        quality: 18,
        width_mm: 310,
        height_mm: 194,
        stream_scale: 1,
        geometry_ready: false,
        decoders: None,
        decoder_epoch: 0,
        selection: None,
    };
    let attachment = blent::attachment::Attachment::new(tokio::sync::watch::channel(settings).0);
    let identity: Arc<dyn ProfileSource> = Arc::new(attachment.clone());
    assert!(identity.profile().identity().is_none());
    attachment.begin_with_transport(
        Some("device:T820".into()),
        Some(blent_config::adb::Transport::Usb),
    );
    let lease = identity.profile();
    assert_eq!(lease.identity(), Some(("T820".into(), "usb")));
    assert!(lease.active());
    let retired = lease.retired();
    tokio::pin!(retired);
    assert!(futures_util::poll!(&mut retired).is_pending());
    attachment.begin_with_transport(
        Some("device:T820".into()),
        Some(blent_config::adb::Transport::Network),
    );
    retired.await;
    assert!(!lease.active());
    assert_eq!(
        identity.profile().identity(),
        Some(("T820".into(), "network"))
    );
}
#[tokio::test]
async fn t820_worker_contract_preserves_success_and_error_without_runtime_handles() {
    let work = CaptureWorkers {
        capture: Box::pin(async { anyhow::bail!("capture failed") }),
        auxiliary: vec![Box::pin(async { Ok(()) })],
    };
    assert_eq!(
        work.capture.await.unwrap_err().to_string(),
        "capture failed"
    );
    for auxiliary in work.auxiliary {
        auxiliary.await.unwrap();
    }
}

#[test]
fn t820_codec_headers_and_refresh_use_independent_ports() {
    let native = blent::media::CodecConfig::default();
    let headers: &dyn HeaderPublication = &native;
    assert!(headers.current().is_none());
    let data = MediaBytes::from_static(b"initial configuration");
    let address = data.as_ptr();
    headers.publish(Some(data));
    assert_eq!(headers.current().unwrap().as_ptr(), address);
    assert_eq!(native.current().unwrap().as_ptr(), address);
    headers.publish(None);
    assert!(headers.current().is_none());
    let native = std::sync::atomic::AtomicBool::new(true);
    let requests: &dyn RefreshRequests = &native;
    assert!(requests.take());
    assert!(!requests.take());
    native.store(true, Ordering::Release);
    assert!(requests.take());
}
