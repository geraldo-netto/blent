//! T1493: bounded opt-in attribution, no capture or shared configuration writes.
use super::*;

fn elapsed<T>(operation: impl FnOnce() -> Option<T>) -> u128 {
    let started = std::time::Instant::now();
    std::hint::black_box(operation().expect("T1493 timed operation failed"));
    started.elapsed().as_micros()
}

pub(in crate::selection::worker::cache) fn observe(
    base: &CaptureConfig,
    scope: &blent::native_scope::Scope,
) -> Vec<serde_json::Value> {
    let executable = std::env::current_exe().unwrap();
    let ffmpeg =
        blent_config::linux::programs::find_in("ffmpeg", &std::env::var_os("PATH").unwrap())
            .unwrap();
    let mut observations = Vec::new();
    for iteration in 0..10 {
        let executable_us = elapsed(|| runtime_identity::file_hash(&executable));
        let helper_us = elapsed(|| runtime_identity::file_hash(&base.helper_path));
        let ffmpeg_identity_us = elapsed(|| runtime_identity::fingerprint(&ffmpeg));
        let platform_us =
            elapsed(|| platform_stamp(Path::new("/proc"), Path::new("/sys"), &base.vaapi_device));
        let version_us = elapsed(|| {
            scope
                .sync_output(
                    std::process::Command::new(&ffmpeg).arg("-version"),
                    blent_config::commands::COMMAND_TIMEOUT,
                )
                .ok()
                .filter(|output| output.status.success())
        });
        let config_us = elapsed(|| Some(blent_config::storage::ConfigStore::default().load()));
        observations.push(serde_json::json!({"iteration":iteration,"executable_us":executable_us,"helper_us":helper_us,"ffmpeg_identity_us":ffmpeg_identity_us,"platform_us":platform_us,"version_us":version_us,"config_us":config_us}));
    }
    observations
}
