use super::*;

#[test]
fn t1671_decoder_honors_filter_capacity_without_frame_threading() {
    for threads in 1..=4 {
        let profile: CameraProfile =
            serde_json::from_value(serde_json::json!({"filter_threads": threads})).unwrap();
        let command = command(Path::new("ffmpeg"), &profile).unwrap();
        let args: Vec<_> = command
            .as_std()
            .get_args()
            .map(|v| v.to_str().unwrap())
            .collect();
        let at = args
            .iter()
            .position(|value| *value == "-filter_threads")
            .unwrap();
        assert_eq!(args[at + 1], threads.to_string());
        let codec_threads: Vec<_> = args
            .windows(2)
            .filter(|pair| pair[0] == "-threads")
            .map(|pair| pair[1])
            .collect();
        assert_eq!(codec_threads, ["1", "1"]);
    }
}

#[test]
fn t1671_filter_change_requires_restart_without_revising_live_image() {
    let profile = CameraProfile::default();
    let session = crate::camera_live::Session::new(profile.clone());
    let before = session.snapshot();
    let mut encoded = serde_json::to_value(profile).unwrap();
    encoded["filter_threads"] = 1.into();
    let changed = serde_json::from_value(encoded).unwrap();
    assert!(session.update(&changed).unwrap().restart_required);
    assert_eq!(session.snapshot(), before);
}

#[test]
fn t1671_invalid_filter_capacity_never_builds_a_decoder_command() {
    for threads in [0, 5, u8::MAX] {
        let profile = CameraProfile {
            filter_threads: threads,
            ..Default::default()
        };
        assert!(command(Path::new("ffmpeg"), &profile).is_err());
    }
}
