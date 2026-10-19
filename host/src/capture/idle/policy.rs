//! One authoritative portable history, selected once for the original worker.
use super::{CaptureConfig, EncoderEvidence};
use blent_config::idle::{self, unchanged, Mode, Sample};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Baseline,
    Trial,
    UnchangedTrial,
    Active,
    Rejected,
}
impl Phase {
    fn adaptive(phase: idle::Phase) -> Self {
        match phase {
            idle::Phase::Baseline => Self::Baseline,
            idle::Phase::Trial => Self::Trial,
            idle::Phase::Active => Self::Active,
            idle::Phase::Rejected => Self::Rejected,
        }
    }
    fn unchanged(phase: unchanged::Phase) -> Self {
        match phase {
            unchanged::Phase::Baseline => Self::Baseline,
            unchanged::Phase::SparseTrial => Self::Trial,
            unchanged::Phase::UnchangedTrial => Self::UnchangedTrial,
            unchanged::Phase::Active => Self::Active,
            unchanged::Phase::Rejected => Self::Rejected,
        }
    }
}

pub(super) enum Policy {
    Adaptive(idle::Policy),
    Unchanged(unchanged::Policy),
}
impl Policy {
    pub(super) fn new(mode: Mode, recipe: bool, decoder: bool) -> Self {
        match mode {
            Mode::Compatible => Self::Adaptive(idle::Policy::new(false)),
            Mode::Adaptive => Self::Adaptive(idle::Policy::new(decoder)),
            Mode::Unchanged => Self::Unchanged(unchanged::Policy::new(recipe, decoder)),
        }
    }
    pub(super) fn phase(&self) -> Phase {
        match self {
            Self::Adaptive(policy) => Phase::adaptive(policy.phase),
            Self::Unchanged(policy) => Phase::unchanged(policy.phase()),
        }
    }
    pub(super) fn interval_ms(&self) -> u32 {
        match self {
            Self::Adaptive(policy) => policy.interval_ms(),
            Self::Unchanged(policy) => policy.interval_ms(),
        }
    }
    pub(super) fn observe(&mut self, conditions: unchanged::Conditions, sample: Sample) {
        match self {
            Self::Adaptive(policy) => policy.observe(sample),
            Self::Unchanged(policy) => policy.observe(conditions, sample),
        }
    }
    pub(super) fn tick(&mut self, conditions: unchanged::Conditions, now_us: u64) {
        match self {
            Self::Adaptive(policy) => policy.tick(now_us),
            Self::Unchanged(policy) => policy.tick(conditions, now_us),
        }
    }
    pub(super) fn reject(&mut self) {
        match self {
            Self::Adaptive(policy) => policy.reject(),
            Self::Unchanged(policy) => policy.reject(),
        }
    }
}

fn codec(name: &str) -> Option<&'static str> {
    match name {
        "libx264" => Some("h264"),
        "libvpx-vp9" => Some("vp9"),
        "libaom-av1" => Some("av1"),
        _ => None,
    }
}

fn decoder(choice: Option<&blent_config::negotiation::DecoderChoice>, expected: &str) -> bool {
    choice.is_some_and(|decoder| {
        !decoder.name.is_empty()
            && decoder.stream.codec == expected
            && decoder.stream.format.depth == 8
    })
}

fn geometry(name: &str, format: (u32, u32, u32, u32, u32)) -> bool {
    let (width, height, fps, bitrate, quality) = format;
    blent_config::raw_frame::Layout::new(width, height, 2).is_ok()
        && blent_config::encoding::Profile::new(name, fps, bitrate, quality).is_ok()
}

/// T1107 checks installed CPU CLI recipe delivery within its recorded finite
/// scope. This admits a trial, never broad hardware/dimension qualification.
pub(super) fn recipe(evidence: &EncoderEvidence) -> bool {
    geometry(&evidence.name, evidence.format)
        && codec(&evidence.name)
            .is_some_and(|expected| decoder(evidence.decoder.as_ref(), expected))
}

pub(super) fn qualified(config: &CaptureConfig, evidence: &EncoderEvidence) -> bool {
    !config.ten_bit && config.encoder == evidence.name && recipe(evidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1115_known_cpu_recipe_admits_current_named_eight_bit_trial() {
        let tracker = crate::latency::LatencyTracker::new();
        let evidence = super::super::tests::evidence(&tracker);
        let config = CaptureConfig {
            encoder: "libx264".into(),
            ..Default::default()
        };
        assert!(
            qualified(&config, &evidence),
            "T1115: eligible current CPU recipe rejected"
        );
    }
}

#[cfg(test)]
mod binding_tests {
    use super::*;
    #[test]
    fn t1115_other_encoder_configuration_cannot_qualify_original_evidence() {
        let tracker = crate::latency::LatencyTracker::new();
        let evidence = super::super::tests::evidence(&tracker);
        let config = CaptureConfig {
            encoder: "h264_nvenc".into(),
            ..Default::default()
        };
        assert!(
            !qualified(&config, &evidence),
            "T1115: original evidence accepted with another encoder configuration"
        );
    }
}

#[cfg(test)]
mod recipe_tests;
