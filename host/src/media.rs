//! Shared media and live encoding settings; no capture or process dependencies.
use crate::media_storage::MediaBytes as Bytes;
use std::sync::Arc;

pub use blent_config::video::Codec;

/// Immutable current codec headers with race-free publication notifications.
/// T405: publishers may run on native encoder threads without a Tokio runtime.
#[derive(Clone)]
pub struct CodecConfig(tokio::sync::watch::Sender<Option<Bytes>>);

impl Default for CodecConfig {
    fn default() -> Self {
        Self::new(None)
    }
}
impl CodecConfig {
    pub fn new(value: Option<Bytes>) -> Self {
        Self(tokio::sync::watch::channel(value).0)
    }
    pub fn current(&self) -> Option<Bytes> {
        self.0.borrow().clone()
    }
    pub fn publish(&self, value: Option<Bytes>) {
        self.0.send_if_modified(|current| {
            if *current == value {
                return false;
            }
            *current = value;
            true
        });
    }
    pub async fn wait_ready(&self) -> Option<Bytes> {
        // Subscribe before examining the value. A publish between a check and
        // suspension stays observable in the watch version; no lost wakeup.
        let mut receiver = self.0.subscribe();
        let ready = receiver.wait_for(|value| value.is_some()).await.ok()?;
        ready.clone()
    }
}

/// One encoded access unit, tagged so the stream server can drop frames
/// safely (resume only at an IDR).
#[derive(Clone)]
pub struct VideoPacket {
    pub data: Bytes,
    pub is_idr: bool,
    /// Allocated across encoder restarts in this daemon instance. Echoed back by the tablet once
    /// the frame is on screen, measuring packet-send-to-render-ack latency.
    pub seq: u32,
    /// Immutable headers for this access unit, never the latest global cache.
    pub codec_config: Option<Bytes>,
    pub generation: Arc<std::sync::atomic::AtomicBool>,
}

/// Retires queued frames when their encoder exits or is cancelled.
pub struct EncoderGeneration {
    pub active: Arc<std::sync::atomic::AtomicBool>,
}

impl Default for EncoderGeneration {
    fn default() -> Self {
        Self {
            active: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }
}

impl EncoderGeneration {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Drop for EncoderGeneration {
    fn drop(&mut self) {
        self.active
            .store(false, std::sync::atomic::Ordering::Release);
    }
}

/// Settings that can change at runtime (from the GUI or the tablet app).
/// A change restarts the encoder; an fps or resolution change also restarts
/// the helper (the EDID is regenerated for the new mode).
#[derive(Clone, Debug, PartialEq)]
pub struct EncoderSettings {
    pub encoder: String,
    pub fps: u32,
    pub bitrate: u32,
    pub width: u32,
    pub height: u32,
    /// Constant-quality target; see `config::DEFAULT_QUALITY`.
    pub quality: u32,
    /// Physical panel size for the generated EDID, in millimetres.
    pub width_mm: u32,
    pub height_mm: u32,
    /// Integer downscale for the stream; see `config::FileConfig::stream_scale`.
    pub stream_scale: u32,
    /// Authenticated tablet metadata has supplied pixel and physical geometry.
    pub geometry_ready: bool,
    pub decoders: Option<DecoderCapabilities>,
    /// Report/selection epochs satisfy 0 <= epoch < u64::MAX. MAX is terminal.
    /// Numeric invalidation never proves original native retirement.
    pub decoder_epoch: u64,
    pub selection: Option<crate::selection::Selected>,
}

pub use blent_config::negotiation::DecoderCapabilities;

impl EncoderSettings {
    /// A terminal settings owner cannot admit another decoder/selection scope.
    pub fn decoder_admission_open(&self) -> bool {
        self.decoder_epoch < u64::MAX
    }

    pub fn decoder_scope(&self) -> Option<String> {
        self.decoder_admission_open()
            .then(|| self.decoder_epoch.to_string())
    }

    /// Immutable identity for a measured selection. Matching below borrows the
    /// existing metadata rather than cloning it on every frame/settings check.
    pub fn selection_key(&self) -> crate::selection::Key {
        crate::selection::Key {
            format: self.selection_format(),
            epoch: self.decoder_epoch,
            decoders: self.decoders.clone(),
        }
    }

    fn selection_format(&self) -> (u32, u32, u32, u32, u32) {
        let (width, height) = self.video_dimensions();
        (width, height, self.fps, self.bitrate, self.quality)
    }

    pub fn selection_matches(&self, key: &crate::selection::Key) -> bool {
        self.decoder_admission_open()
            && self.encoder == "auto"
            && self.geometry_ready
            && key.format == self.selection_format()
            && key.epoch == self.decoder_epoch
            && key.decoders == self.decoders
    }

    pub fn calibrating(&self) -> bool {
        if !self.decoder_admission_open()
            || self.encoder != "auto"
            || cfg!(feature = "inproc-encoder")
        {
            return false;
        }
        if self
            .decoders
            .as_ref()
            .is_some_and(|caps| caps.protocol != 2)
        {
            return false;
        }
        self.selection
            .as_ref()
            .filter(|s| self.selection_matches(&s.key))
            .is_none_or(|s| s.calibrating)
    }

    pub fn selected_workers(&self) -> u32 {
        self.selection
            .as_ref()
            .filter(|s| self.selection_matches(&s.key))
            .map_or(0, |s| s.workers)
    }

    pub fn same_stream(&self, other: &Self) -> bool {
        self.effective_encoder() == other.effective_encoder()
            && self.selected_workers() == other.selected_workers()
            && self.decoder_choice() == other.decoder_choice()
            && self.helper_geometry() == other.helper_geometry()
            && (self.bitrate, self.quality, self.geometry_ready)
                == (other.bitrate, other.quality, other.geometry_ready)
    }
    pub fn video_dimensions(&self) -> (u32, u32) {
        let scale = self.stream_scale.max(1);
        (
            ((self.width / scale) & !1).max(2),
            ((self.height / scale) & !1).max(2),
        )
    }
    pub fn clear_decoders(&mut self) {
        self.decoders = None;
        self.selection = None;
        self.decoder_epoch = self.decoder_epoch.saturating_add(1);
    }
    pub fn selection_reason(&self) -> &str {
        if !self.decoder_admission_open() {
            return "Decoder metadata generations exhausted; restart Blent for new profile selection";
        }
        if self.encoder != "auto" {
            return if self.effective_encoder() == self.encoder {
                "Explicit encoder preference"
            } else {
                "H.264 fallback: requested codec lacks current peer compatibility"
            };
        }
        if cfg!(feature = "inproc-encoder") {
            return "Automatic calibration requires the CLI build; using libx264";
        }
        self.selection
            .as_ref()
            .filter(|s| self.selection_matches(&s.key))
            .map(|s| s.reason.as_str())
            .unwrap_or("H.264 fallback while awaiting compatible encoder measurements")
    }
    pub fn decoder_choice(&self) -> Option<&blent_config::negotiation::DecoderChoice> {
        self.selection
            .as_ref()
            .filter(|s| self.selection_matches(&s.key))?
            .decoder
            .as_ref()
    }
    pub fn effective_encoder(&self) -> &str {
        if self.encoder == "auto" {
            return self
                .selection
                .as_ref()
                .filter(|s| self.selection_matches(&s.key))
                .map(|s| s.encoder.as_str())
                .unwrap_or("libx264");
        }
        let codec = Codec::from_encoder(&self.encoder);
        if !codec.framed() || self.decoder_supports(codec) {
            &self.encoder
        } else {
            "libx264"
        }
    }
    pub fn decoder_supports(&self, codec: Codec) -> bool {
        let Some(caps) = &self.decoders else {
            return false;
        };
        let Some(scope) = self.decoder_scope() else {
            return false;
        };
        let (width, height) = self.video_dimensions();
        caps.matches(&scope, width, height, self.fps)
            && caps.codecs.iter().any(|name| name == codec.wire_name())
    }

    pub fn helper_geometry(&self) -> (u32, u32, u32, u32, u32, u32) {
        (
            self.width,
            self.height,
            self.width_mm,
            self.height_mm,
            self.fps,
            self.stream_scale,
        )
    }
}

#[cfg(test)]
#[path = "media/exhaustion_tests.rs"]
mod exhaustion_tests;

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn selection_fixture() -> EncoderSettings {
        EncoderSettings {
            encoder: "auto".into(),
            fps: 60,
            bitrate: 20_000,
            width: 1280,
            height: 800,
            quality: 18,
            width_mm: 310,
            height_mm: 194,
            stream_scale: 1,
            geometry_ready: true,
            decoders: None,
            decoder_epoch: 1,
            selection: None,
        }
    }

    #[test]
    fn t821_selection_contract_identity_and_invalidation_have_one_policy_owner() {
        let mut settings = selection_fixture();
        let key = settings.selection_key();
        assert!(settings.selection_matches(&key));
        settings.selection = Some(crate::selection::Selected {
            key: key.clone(),
            workers: 3,
            encoder: "h264_vaapi".into(),
            reason: "measured".into(),
            verified: true,
            calibrating: false,
            decoder: None,
        });
        assert_eq!(settings.selected_workers(), 3);
        assert_eq!(settings.effective_encoder(), "h264_vaapi");
        assert!(!settings.calibrating());
        settings.decoder_epoch += 1;
        assert!(!settings.selection_matches(&key));
        assert_eq!(settings.selected_workers(), 0);
        assert_eq!(settings.effective_encoder(), "libx264");
        assert_eq!(settings.calibrating(), !cfg!(feature = "inproc-encoder"));
        for mutate in profile_mutations().into_iter().chain(geometry_mutations()) {
            let mut changed = selection_fixture();
            mutate(&mut changed);
            let expected = changed.encoder == "auto"
                && changed.geometry_ready
                && changed.selection_key() == key;
            assert_eq!(changed.selection_matches(&key), expected);
        }
        settings = selection_fixture();
        settings.decoders = Some(DecoderCapabilities::default());
        assert!(!settings.selection_matches(&key));
        assert!(settings.selection_matches(&settings.selection_key()));
    }

    #[test]
    fn t821_bounded_key_properties_preserve_dimensions_at_unsigned_boundaries() {
        for width in [0, 1, 2, 4095, 4096, u32::MAX] {
            for scale in [0, 1, 2, 4096, u32::MAX] {
                let mut settings = selection_fixture();
                settings.width = width;
                settings.height = width;
                settings.stream_scale = scale;
                settings.fps = width;
                settings.bitrate = width;
                settings.quality = width;
                settings.decoder_epoch = u64::MAX - 1;
                let key = settings.selection_key();
                assert!(settings.selection_matches(&key));
                assert_eq!((key.format.0, key.format.1), settings.video_dimensions());
                assert_eq!(key.format.0 % 2, 0);
                assert!(key.format.0 >= 2);
                settings.clear_decoders();
                assert!(!settings.selection_matches(&key));
            }
        }
    }

    #[test]
    fn t497_restart_identity_tracks_stream_changes_but_not_inventory_metadata() {
        let base = EncoderSettings {
            encoder: "libx264".into(),
            fps: 60,
            bitrate: 20_000,
            width: 1280,
            height: 800,
            quality: 18,
            width_mm: 310,
            height_mm: 194,
            stream_scale: 1,
            geometry_ready: true,
            decoders: None,
            decoder_epoch: 0,
            selection: None,
        };
        let mut metadata = base.clone();
        metadata.decoder_epoch = u64::MAX;
        assert!(base.same_stream(&metadata));
        for mutate in profile_mutations().into_iter().chain(geometry_mutations()) {
            let mut other = base.clone();
            mutate(&mut other);
            assert!(!base.same_stream(&other), "T497 lost restart: {other:?}");
        }
    }

    fn profile_mutations() -> [fn(&mut EncoderSettings); 4] {
        [
            |s| s.encoder = "h264_vaapi".into(),
            |s| s.fps = 30,
            |s| s.bitrate += 1,
            |s| s.quality += 1,
        ]
    }

    fn geometry_mutations() -> [fn(&mut EncoderSettings); 5] {
        [
            |s| s.width += 2,
            |s| s.height += 2,
            |s| s.width_mm += 1,
            |s| s.height_mm += 1,
            |s| s.geometry_ready = false,
        ]
    }

    #[test]
    fn t401_delayed_packet_lease_keeps_storage_after_producer_retirement() {
        use std::sync::atomic::Ordering;
        let owner = EncoderGeneration::new();
        let packet = VideoPacket {
            data: Bytes::from(vec![0, 0, 1, 0x65, 0x80]),
            is_idr: true,
            seq: u32::MAX,
            codec_config: Some(Bytes::from(vec![0, 0, 1, 0x67, 0x42])),
            generation: owner.active.clone(),
        };
        let delayed = packet.clone();
        let storage = packet.data.as_ptr();
        let configuration = packet.codec_config.as_ref().unwrap().as_ptr();
        drop(packet);
        drop(owner);
        let replacement = EncoderGeneration::new();

        // Retirement forbids sending; it must not invalidate memory held by
        // slow consumers, or substitute a replacement encoder's CSD/epoch.
        assert!(!delayed.generation.load(Ordering::Acquire));
        assert!(replacement.active.load(Ordering::Acquire));
        assert!(!Arc::ptr_eq(&delayed.generation, &replacement.active));
        assert_eq!(delayed.data.as_ptr(), storage);
        assert_eq!(delayed.data.as_ref(), &[0, 0, 1, 0x65, 0x80]);
        let config = delayed.codec_config.as_ref().unwrap();
        assert_eq!(config.as_ptr(), configuration);
        assert_eq!(config.as_ref(), &[0, 0, 1, 0x67, 0x42]);
        assert_eq!(delayed.seq, u32::MAX);
    }

    #[test]
    fn codec_is_picked_from_the_encoder_name() {
        assert_eq!(Codec::from_encoder("h264_nvenc"), Codec::H264);
        assert_eq!(Codec::from_encoder("libx264"), Codec::H264);
        assert_eq!(Codec::from_encoder("hevc_nvenc"), Codec::Hevc);
        assert_eq!(Codec::from_encoder("hevc_vaapi"), Codec::Hevc);
    }
}

#[cfg(test)]
mod codec_config_tests {
    use super::*;

    #[tokio::test]
    async fn t405_headers_published_without_waiters_survive_clone_and_reset() {
        let headers = CodecConfig::default();
        headers.publish(Some(Bytes::from(vec![1, 2, 3])));
        let next = headers.clone();
        drop(headers);
        let previous = next.wait_ready().await.unwrap();
        next.publish(None);
        assert!(next.current().is_none());
        next.publish(Some(Bytes::from(vec![4, 5])));
        assert_eq!(next.wait_ready().await.unwrap().as_ref(), &[4, 5]);
        assert_eq!(previous.as_ref(), &[1, 2, 3]);
    }
}
