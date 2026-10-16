//! Environment configuration adapter; the interval contract stays portable.
pub(super) fn configured_interval() -> anyhow::Result<blent_config::video::liveness::Interval> {
    use blent_config::video::liveness;
    liveness::interval(blent_config::retirement_limits::configured(
        "BLENT_VIDEO_LIVENESS_MS",
        liveness::DEFAULT_INTERVAL_MS,
        liveness::MAX_INTERVAL_MS,
    )?)
}
