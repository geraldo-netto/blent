//! Encoder policy shared by the FFmpeg CLI and optional libavcodec adapters.
//! No FFmpeg dependency: adapters translate these values through stock public APIs.
use anyhow::{Context, Result};

pub mod windows;

/// Same measured selection order on every platform. Hardware refers to the
/// matched tablet decoder, not an advertised encoder name.
pub fn rank(
    fps: f64,
    hardware: bool,
    p95_us: u64,
    first_us: u64,
    required_fps: u32,
) -> (bool, bool, u64, u64) {
    (fps < f64::from(required_fps), !hardware, p95_us, first_us)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Backend {
    Nvenc,
    Vaapi,
    X264,
    Vpx,
    Aom,
}

#[derive(Clone, Copy, Debug)]
pub struct Encoder {
    pub name: &'static str,
    pub label: &'static str,
    pub backend: Backend,
    pub hevc: bool,
}

pub const ENCODERS: [Encoder; 11] = [
    Encoder {
        name: "libaom-av1",
        label: "Software AV1 (libaom)",
        backend: Backend::Aom,
        hevc: false,
    },
    Encoder {
        name: "av1_nvenc",
        label: "NVIDIA AV1 (NVENC)",
        backend: Backend::Nvenc,
        hevc: false,
    },
    Encoder {
        name: "av1_vaapi",
        label: "Intel / AMD AV1 (VAAPI)",
        backend: Backend::Vaapi,
        hevc: false,
    },
    Encoder {
        name: "h264_nvenc",
        label: "NVIDIA H.264 (NVENC)",
        backend: Backend::Nvenc,
        hevc: false,
    },
    Encoder {
        name: "hevc_nvenc",
        label: "NVIDIA HEVC (NVENC)",
        backend: Backend::Nvenc,
        hevc: true,
    },
    Encoder {
        name: "h264_vaapi",
        label: "AMD / Intel H.264 (VAAPI)",
        backend: Backend::Vaapi,
        hevc: false,
    },
    Encoder {
        name: "h264_vaapi_baseline",
        label: "AMD / Intel H.264 low latency (VAAPI)",
        backend: Backend::Vaapi,
        hevc: false,
    },
    Encoder {
        name: "hevc_vaapi",
        label: "AMD / Intel HEVC (VAAPI)",
        backend: Backend::Vaapi,
        hevc: true,
    },
    Encoder {
        name: "libx264",
        label: "Software (libx264)",
        backend: Backend::X264,
        hevc: false,
    },
    Encoder {
        name: "libvpx-vp9",
        label: "Software VP9 (libvpx)",
        backend: Backend::Vpx,
        hevc: false,
    },
    Encoder {
        name: "vp9_vaapi",
        label: "Intel / AMD VP9 (VAAPI)",
        backend: Backend::Vaapi,
        hevc: false,
    },
];

pub fn canonical_name(name: &str) -> &str {
    match name {
        "vaapih264enc" => "h264_vaapi",
        _ => name,
    }
}

/// A selectable profile is not necessarily a distinct FFmpeg encoder.
pub fn ffmpeg_name(name: &str) -> &str {
    match canonical_name(name) {
        "h264_vaapi_baseline" => "h264_vaapi",
        other => other,
    }
}

pub fn find(name: &str) -> Option<&'static Encoder> {
    let canonical = canonical_name(name);
    ENCODERS.iter().find(|encoder| encoder.name == canonical)
}

pub const INPROC_VAAPI_UNSUPPORTED: &str = "VAAPI is unavailable in this in-process build for this profile: only h264_vaapi_baseline has a validated hardware-frame adapter. Build without --features inproc-encoder for other VAAPI profiles, or select libx264/NVENC.";

impl Encoder {
    pub fn validate_inproc(&self) -> Result<()> {
        anyhow::ensure!(
            self.backend != Backend::Vaapi || self.name == "h264_vaapi_baseline",
            INPROC_VAAPI_UNSUPPORTED
        );
        anyhow::ensure!(
            !crate::video::Codec::from_encoder(self.name).framed(),
            "VP9/AV1 require the stock FFmpeg CLI adapter; build without inproc-encoder"
        );
        Ok(())
    }
}

pub struct Profile {
    pub encoder: &'static Encoder,
    /// A nominal second of captured frames. Each adapter has its own idle/join IDR mechanism.
    pub gop: u32,
    pub max_rate_bps: u64,
    bitrate_kbps: u32,
    quality: u32,
    buffer_kbits: Option<u32>,
    workers: u32,
}

fn validate_numbers(fps: u32, bitrate_kbps: u32, quality: u32) -> Result<()> {
    anyhow::ensure!((1..=crate::MAX_FPS).contains(&fps), "Invalid encoder FPS");
    anyhow::ensure!(
        (1..=crate::MAX_BITRATE_KBPS).contains(&bitrate_kbps),
        "Invalid encoder bitrate"
    );
    anyhow::ensure!(
        (crate::MIN_QUALITY..=crate::MAX_QUALITY).contains(&quality),
        "Invalid encoder quality"
    );
    Ok(())
}

impl Profile {
    /// Low-level profiles accept positive rates up to the shared configuration
    /// ceilings, including small replay bitrates below the UI minimum. Quality
    /// uses the shared supported range. Invalid inputs are rejected, not clamped.
    pub fn new(name: &str, fps: u32, bitrate_kbps: u32, quality: u32) -> Result<Self> {
        let encoder = find(name).with_context(|| format!("Unknown encoder: {name}"))?;
        validate_numbers(fps, bitrate_kbps, quality)?;
        let gop = fps;
        let buffer_kbits = match encoder.backend {
            Backend::Nvenc => Some((bitrate_kbps / gop).max(200)),
            Backend::X264 => Some((bitrate_kbps * 2 / gop).max(200)),
            Backend::Vaapi | Backend::Vpx | Backend::Aom => None,
        };
        Ok(Self {
            encoder,
            gop,
            max_rate_bps: u64::from(bitrate_kbps) * 1000,
            bitrate_kbps,
            quality,
            buffer_kbits,
            workers: 1,
        })
    }

    pub fn with_workers(mut self, count: u32) -> Result<Self> {
        self.workers = crate::encoder_workers::validate(count)?.max(1);
        Ok(self)
    }

    fn codec_options(&self) -> Vec<(&'static str, String)> {
        let (static_options, quality_key): (&[(&str, &str)], _) = match self.encoder.backend {
            Backend::Nvenc => (
                &[
                    ("preset", "p1"),
                    ("tune", "ull"),
                    ("zerolatency", "1"),
                    ("delay", "0"),
                    ("rc", "vbr"),
                    ("multipass", "0"),
                    ("rc-lookahead", "0"),
                    ("forced-idr", "1"),
                ],
                "cq",
            ),
            Backend::Vaapi => (&[("rc_mode", "CQP")], "qp"),
            Backend::X264 => (
                &[
                    ("preset", "ultrafast"),
                    ("tune", "zerolatency"),
                    ("threads", "1"),
                ],
                "crf",
            ),
            Backend::Aom => (
                &[
                    ("usage", "realtime"),
                    ("cpu-used", "8"),
                    ("lag-in-frames", "0"),
                    ("auto-alt-ref", "0"),
                    ("row-mt", "1"),
                ],
                "crf",
            ),
            Backend::Vpx => (
                &[
                    ("deadline", "realtime"),
                    ("cpu-used", "8"),
                    ("lag-in-frames", "0"),
                    ("auto-alt-ref", "0"),
                    ("row-mt", "1"),
                ],
                "crf",
            ),
        };
        let mut options = static_options
            .iter()
            .map(|&(key, value)| (key, value.into()))
            .collect::<Vec<_>>();
        if let Some((_, count)) = options.iter_mut().find(|(key, _)| *key == "threads") {
            *count = self.workers.to_string();
        }
        options.push((quality_key, self.quality.to_string()));
        options
    }

    /// CLI syntax and explicit CLI-only controls. The caller limits ten_bit to HEVC.
    pub fn cli_options(&self, ten_bit: bool) -> Vec<(String, String)> {
        let mut options = self
            .codec_options()
            .into_iter()
            .map(|(key, value)| (format!("-{key}"), value))
            .collect::<Vec<_>>();
        // VAAPI CQP retains this existing option but does not enforce a bitrate ceiling (T259).
        options.push(("-maxrate".into(), format!("{}k", self.bitrate_kbps)));
        options.push(("-g".into(), self.gop.to_string()));
        if let Some(buffer) = self.buffer_kbits {
            options.push(("-bufsize".into(), format!("{buffer}k")));
        }
        if matches!(self.encoder.backend, Backend::Vpx | Backend::Aom) {
            options.push(("-b:v".into(), format!("{}k", self.bitrate_kbps)));
        }
        self.cli_controls(ten_bit, &mut options);
        options
    }

    fn cli_controls(&self, ten_bit: bool, options: &mut Vec<(String, String)>) {
        let controls: &[(&str, &str)] = match self.encoder.backend {
            Backend::Nvenc => &[("-bf", "0"), ("-b:v", "0")],
            // T400: request processing depth one when the CLI capability probe
            // confirms support; avoids a measured extra input interval.
            Backend::Vaapi if crate::video::Codec::from_encoder(self.encoder.name).framed() => {
                &[("-bf", "0"), ("-async_depth", "1")]
            }
            Backend::Vaapi => &[("-bf", "0"), ("-idr_interval", "0"), ("-async_depth", "1")],
            Backend::X264 => &[("-x264-params", "scenecut=0")],
            Backend::Vpx | Backend::Aom => &[("-pix_fmt", "yuv420p")],
        };
        options.extend(
            controls
                .iter()
                .map(|&(key, value)| (key.into(), value.into())),
        );
        self.cli_profile(ten_bit, options);
    }

    fn cli_profile(&self, ten_bit: bool, options: &mut Vec<(String, String)>) {
        for &(key, value) in self.baseline_profile() {
            let key = if key == "profile" {
                "-profile:v"
            } else {
                "-coder"
            };
            options.push((key.into(), value.into()));
        }
        if self.encoder.backend == Backend::Nvenc && ten_bit {
            options.push(("-profile:v".into(), "main10".into()));
        }
    }

    fn baseline_profile(&self) -> &'static [(&'static str, &'static str)] {
        if self.encoder.name == "h264_vaapi_baseline" {
            &[("profile", "constrained_baseline"), ("coder", "cavlc")]
        } else {
            &[]
        }
    }

    /// Libavcodec receives rate/GOP/B-frame/color fields through its typed context.
    /// CPU input remains 8-bit NV12; baseline VAAPI uploads to owned surfaces.
    pub fn inproc_options(&self) -> Result<Vec<(&'static str, String)>> {
        self.encoder.validate_inproc()?;
        let mut options = self.codec_options();
        options.extend(
            self.baseline_profile()
                .iter()
                .map(|&(key, value)| (key, value.into())),
        );
        if self.encoder.backend == Backend::Vaapi {
            options.extend([("async_depth", "1".into()), ("idr_interval", "0".into())]);
        }
        if let Some(buffer) = self.buffer_kbits {
            options.push(("bufsize", (u64::from(buffer) * 1000).to_string()));
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t839_profile_rejects_bitrate_overflow_before_arithmetic() {
        assert!(Profile::new("libx264", 60, u32::MAX, 20).is_err());
    }

    #[test]
    fn t839_numeric_boundaries_match_low_level_profile_contract() {
        // Low-level replay profiles may use less than the UI bitrate minimum.
        let rates = [
            0,
            1,
            200,
            500,
            crate::MIN_BITRATE_KBPS,
            crate::MAX_BITRATE_KBPS,
            crate::MAX_BITRATE_KBPS + 1,
            u32::MAX / 2,
            u32::MAX / 2 + 1,
            u32::MAX,
        ];
        let frames = [
            0,
            1,
            crate::MIN_FPS,
            crate::MAX_FPS,
            crate::MAX_FPS + 1,
            u32::MAX,
        ];
        let qualities = [
            0,
            crate::MIN_QUALITY - 1,
            crate::MIN_QUALITY,
            crate::MAX_QUALITY,
            crate::MAX_QUALITY + 1,
            u32::MAX,
        ];
        for encoder in ENCODERS {
            for fps in frames {
                for bitrate in rates {
                    for quality in qualities {
                        let expected = (1..=crate::MAX_FPS).contains(&fps)
                            && (1..=crate::MAX_BITRATE_KBPS).contains(&bitrate)
                            && (crate::MIN_QUALITY..=crate::MAX_QUALITY).contains(&quality);
                        let result = Profile::new(encoder.name, fps, bitrate, quality);
                        assert_eq!(
                            result.is_ok(),
                            expected,
                            "T839: {} fps={fps} bitrate={bitrate} quality={quality}",
                            encoder.name
                        );
                        if let Ok(profile) = result {
                            assert_eq!(profile.gop, fps);
                            assert_eq!(profile.max_rate_bps, u64::from(bitrate) * 1000);
                            assert!(!profile.cli_options(false).is_empty());
                        }
                    }
                }
            }
        }
        assert!(Profile::new("unknown", 60, 20000, 18).is_err());
    }

    #[test]
    fn t787_shared_vaapi_preserves_explicit_low_latency_policy() {
        let profile = Profile::new("h264_vaapi_baseline", 60, 20000, 18).unwrap();
        let options = profile
            .inproc_options()
            .expect("T787: hardware-frame adapter admits baseline VAAPI");
        for (key, value) in [
            ("profile", "constrained_baseline"),
            ("coder", "cavlc"),
            ("async_depth", "1"),
            ("idr_interval", "0"),
            ("rc_mode", "CQP"),
            ("qp", "18"),
        ] {
            assert!(options.contains(&(key, value.into())));
        }
        for name in ["h264_vaapi", "hevc_vaapi", "av1_vaapi", "vp9_vaapi"] {
            assert!(Profile::new(name, 60, 20000, 18)
                .unwrap()
                .inproc_options()
                .is_err());
        }
    }

    #[test]
    fn t612_manual_workers_use_shared_policy_and_preserve_default() {
        for workers in [0, 1, 2, 4, 128] {
            let profile = Profile::new("libx264", 60, 20000, 18)
                .unwrap()
                .with_workers(workers)
                .unwrap();
            assert!(profile
                .cli_options(false)
                .contains(&("-threads".into(), workers.max(1).to_string())));
            assert!(profile
                .inproc_options()
                .unwrap()
                .contains(&("threads", workers.max(1).to_string())));
        }
        assert!(Profile::new("libx264", 60, 20000, 18)
            .unwrap()
            .with_workers(129)
            .is_err());
        let profile = Profile::new("h264_vaapi", 60, 20000, 18)
            .unwrap()
            .with_workers(4)
            .unwrap();
        assert!(!profile
            .cli_options(false)
            .iter()
            .any(|(key, _)| key == "-threads"));
    }

    #[test]
    fn t613_x264_defaults_to_one_worker_in_both_adapters() {
        for encoder in ENCODERS {
            let profile = Profile::new(encoder.name, 60, 20000, 18).unwrap();
            let expected = (encoder.backend == Backend::X264).then_some("1");
            let cli = profile.cli_options(false);
            assert_eq!(
                cli.iter()
                    .find(|(key, _)| key == "-threads")
                    .map(|(_, value)| value.as_str()),
                expected
            );
            if let Ok(options) = profile.inproc_options() {
                assert_eq!(
                    options
                        .iter()
                        .find(|(key, _)| *key == "threads")
                        .map(|(_, value)| value.as_str()),
                    expected
                );
            }
        }
    }

    #[test]
    fn t433_av1_profiles_use_framed_stock_cli_and_reject_inproc() {
        for name in ["libaom-av1", "av1_nvenc", "av1_vaapi"] {
            let profile = Profile::new(name, 60, 20000, 18).expect("T433: AV1 encoder missing");
            let codec = crate::video::Codec::from_encoder(name);
            assert_eq!(codec.wire_name(), "av1");
            assert_eq!(codec.muxer(), "ivf");
            assert!(codec.framed());
            assert!(profile.inproc_options().is_err());
            assert!(!profile.encoder.hevc);
        }
    }

    #[test]
    fn t400_low_latency_vaapi_is_distinct_from_stock_encoder_name() {
        let profile = Profile::new("h264_vaapi_baseline", 60, 20000, 18)
            .expect("T400: explicit low-latency profile must be selectable");
        assert_eq!(profile.encoder.backend, Backend::Vaapi);
        assert!(!profile.encoder.hevc);
        assert!(profile.encoder.validate_inproc().is_ok());
        assert_eq!(
            crate::ffmpeg_encoder_name("h264_vaapi_baseline"),
            "h264_vaapi"
        );
        let options = profile.cli_options(false);
        assert!(options.contains(&("-profile:v".into(), "constrained_baseline".into())));
        assert!(options.contains(&("-coder".into(), "cavlc".into())));
        assert!(!Profile::new("h264_vaapi", 60, 20000, 18)
            .unwrap()
            .cli_options(false)
            .iter()
            .any(|(key, _)| key == "-profile:v"));
    }

    #[test]
    fn t373_registry_preserves_aliases_and_adapter_capabilities() {
        assert_eq!(find("vaapih264enc").unwrap().name, "h264_vaapi");
        assert!(find("unknown").is_none());
        for encoder in ENCODERS {
            let inproc = ["libx264", "h264_nvenc", "hevc_nvenc", "h264_vaapi_baseline"]
                .contains(&encoder.name);
            assert_eq!(encoder.hevc, encoder.name.starts_with("hevc"));
            assert_eq!(encoder.validate_inproc().is_ok(), inproc);
            for fps in [1, 10, 90] {
                let profile = Profile::new(encoder.name, fps, 60000, 32).unwrap();
                assert_eq!(profile.gop, fps);
                assert_eq!(profile.max_rate_bps, 60000000);
                assert_eq!(profile.inproc_options().is_ok(), inproc);
            }
        }
    }
}
