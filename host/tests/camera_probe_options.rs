//! T1203: compile and exercise the actual maintained probe argument schema.
#[path = "../examples/camera-feedback-probe/options.rs"]
mod options;
#[path = "../examples/camera-feedback-probe/samples.rs"]
mod samples;
use clap::Parser;
use options::Options;

#[test]
fn t1203_obsolete_v4l2_output_flags_cannot_enter_the_probe() {
    for flag in ["--front-device", "--rear-device"] {
        assert!(
            Options::try_parse_from(["probe", flag, "/tmp/unused"]).is_err(),
            "T1203 obsolete output flag admitted: {flag}"
        );
    }
}

#[test]
fn t1203_current_profile_and_measurement_capacity_round_trip() {
    let parsed = Options::try_parse_from([
        "probe",
        "--lens",
        "rear",
        "--width",
        "640",
        "--height",
        "480",
        "--fps",
        "30",
        "--seconds",
        "1",
        "--stall-feedback-ms",
        "1000",
        "--max-samples",
        "2",
    ])
    .unwrap();
    assert_eq!(parsed.camera.lens, blent_config::camera::Lens::Rear);
    assert_eq!(parsed.camera.width, 640);
    assert_eq!(parsed.camera.height, 480);
    assert_eq!(parsed.camera.fps, 30);
    assert_eq!(parsed.seconds, 1);
    assert_eq!(parsed.stall_feedback_ms, 1000);
    let mut samples = samples::Samples::new(parsed.max_samples).unwrap();
    samples.record(std::time::Duration::from_millis(1)).unwrap();
    samples.record(std::time::Duration::from_millis(2)).unwrap();
    assert!(samples.record(std::time::Duration::ZERO).is_err());
    assert_eq!(samples.count(), 2);
    assert_eq!(samples.first(), Some(1));
    assert_eq!(samples.timestamps(), [1, 2]);
}
