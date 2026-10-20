use blent_config::camera::CameraProfile;
use clap::Parser;

#[derive(Parser)]
pub(super) struct Options {
    #[command(flatten)]
    pub(super) camera: CameraProfile,
    #[arg(long, default_value_t = 10)]
    pub(super) seconds: u64,
    #[arg(long, default_value_t = 0)]
    pub(super) stall_feedback_ms: u64,
    /// Timestamp admission cap; exhaustion fails the measurement.
    #[arg(long, default_value_t = super::samples::DEFAULT_CAPACITY)]
    pub(super) max_samples: usize,
}
