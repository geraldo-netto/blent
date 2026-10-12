use super::*;
use std::ffi::OsString;
fn arguments(values: &[&str]) -> impl Iterator<Item = OsString> {
    values
        .iter()
        .map(|v| OsString::from(v))
        .collect::<Vec<_>>()
        .into_iter()
}
#[test]
fn t1013_helper_arguments_are_bounded_before_native_acquisition() {
    assert!(dispatch(arguments(&[])).is_none());
    assert!(dispatch(arguments(&["other"])).is_none());
    for values in [
        vec![],
        vec!["0", "1"],
        vec!["1000000001", "1"],
        vec!["1", "1000001"],
        vec!["1", "1", "extra"],
        vec!["-1", "1"],
        vec!["1", "-1"],
        vec!["18446744073709551616", "1"],
    ] {
        assert!(Options::parse(arguments(&values)).is_err());
    }
    for interval in ["1", "250000000", "1000000000"] {
        assert!(Options::parse(arguments(&[interval, "0"])).is_ok());
    }
    assert!(dispatch(arguments(&["--blent-activity-dpms", "0", "1"]))
        .unwrap()
        .is_err());
}
#[test]
fn t1013_retained_native_producer_and_output_failures_preserve_framing() {
    let device = device::tests::producer_device();
    let mut output = Vec::new();
    produce(
        &device,
        &mut output,
        Options {
            interval: Duration::from_millis(1),
            samples: 2,
        },
    )
    .unwrap();
    let lines = output
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert!(lines
        .iter()
        .all(|line| DisplayObservation::decode(line).unwrap().on == Some(true)));
    let mut rejecting = std::io::Cursor::new([0u8; 0]);
    assert!(publish(
        &mut rejecting,
        DisplayObservation {
            supported: true,
            on: Some(true)
        }
    )
    .is_err());
}
#[test]
fn t1013_runtime_loader_reports_missing_libraries_symbols_and_abi() {
    use library::Library;
    assert!(Library::open(c"__blent_missing_x11_library__").is_err());
    let library = Library::open(c"libX11.so.6").unwrap();
    type Open = unsafe extern "C" fn(*const std::ffi::c_char) -> *mut std::ffi::c_void;
    assert!(unsafe { library.symbol::<Open>(c"XOpenDisplay") }.is_ok());
    assert!(unsafe { library.symbol::<Open>(c"__blent_missing_x11_symbol__") }.is_err());
    assert!(unsafe { library.symbol::<u8>(c"XOpenDisplay") }.is_err());
}
#[test]
fn t1013_actual_xlib_private_server_and_missing_display_report_native_capability() {
    const TEST: &str = "dpms_probe::tests::t1013_actual_xlib_private_server_and_missing_display_report_native_capability";
    if std::env::var_os("BLENT_T1013_XLIB").is_none() {
        let output = std::process::Command::new("xvfb-run")
            .args([
                "-a",
                "-s",
                "-screen 0 64x64x24 -nolisten tcp -extension DPMS",
            ])
            .arg(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env("BLENT_T1013_XLIB", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    assert!(!Device::open().unwrap().sample().unwrap().supported);
    dispatch(arguments(&["--blent-activity-dpms", "1", "1"]))
        .unwrap()
        .unwrap();
    std::env::set_var("DISPLAY", "invalid-display");
    assert!(dispatch(arguments(&["--blent-activity-dpms", "1", "1"]))
        .unwrap()
        .is_err());
}
