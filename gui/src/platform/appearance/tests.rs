use super::*;

const FIXTURE: &str = r#"
source = "Fixture"
background = [248, 248, 249]
surface = [255, 255, 255]
foreground = [32, 32, 32]
accent = [31, 158, 222]
selected_text = [255, 255, 255]
font_family = "Sans"
font_points = 10.0
text_scale = 1.0
animations = false
"#;

#[test]
fn t1786_native_contract_rejects_malformed_and_out_of_range_inputs() {
    assert_eq!(parse(FIXTURE.as_bytes(), true).unwrap().font_points, 10.0);
    for bytes in [vec![], vec![255], vec![b' '; 8193]] { assert!(parse(&bytes, true).is_err()); }
    assert!(parse(FIXTURE.as_bytes(), false).is_err());
    for replacement in ["font_points = nan", "font_points = inf", "font_points = 0.0", "font_points = 49.0", "font_points = 10"] {
        assert!(parse(FIXTURE.replace("font_points = 10.0", replacement).as_bytes(), true).is_err());
    }
    for replacement in ["background = [0, 0, 0]", "background = [256, 0, 0]", "background = [-1, 0, 0]", "background = [0, 0]", "background = [0, 0, 1.0]"] {
        assert!(parse(FIXTURE.replace("background = [248, 248, 249]", replacement).as_bytes(), true).is_err());
    }
}

#[test]
fn t1786_native_contract_requires_bounded_names_and_typed_fields() {
    let value: toml::Value = FIXTURE.parse().unwrap();
    for name in ["", "\n", &"x".repeat(257)] {
        let mut changed = value.clone();
        changed["source"] = toml::Value::String(name.into());
        assert!(parse(changed.to_string().as_bytes(), true).is_err());
    }
    for key in value.as_table().unwrap().keys() {
        let mut changed = value.clone();
        changed.as_table_mut().unwrap().remove(key);
        assert!(parse(toml::to_string(&changed).unwrap().as_bytes(), true).is_err(), "{key}");
    }
    for scale in [0.49, 4.01, f64::INFINITY, f64::NAN] {
        let mut changed = value.clone();
        changed["text_scale"] = toml::Value::Float(scale);
        assert!(parse(toml::to_string(&changed).unwrap().as_bytes(), true).is_err());
    }
}

#[test]
fn t1786_font_input_is_regular_valid_and_bounded() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("font");
    assert!(read_font(&path).is_err());
    assert!(read_font(root.path()).is_err());
    std::fs::write(&path, []).unwrap();
    assert!(read_font(&path).is_err());
    std::fs::write(&path, "invalid-font").unwrap();
    assert!(read_font(&path).is_err());
    std::fs::File::create(&path).unwrap().set_len(8 * 1024 * 1024 + 1).unwrap();
    assert!(read_font(&path).is_err());
    assert!(!font("sans").unwrap().is_empty());
}

#[test]
fn t1786_native_gtk_probe_on_private_display() {
    let output = Command::new("xvfb-run").args(["-a", "/usr/bin/python3", "-I", "-c", include_str!("gtk.py")])
        .output_bounded().unwrap();
    let appearance = parse(&output.stdout, output.status.success()).unwrap();
    assert!(appearance.source.starts_with("GTK "));
    assert!(!font(&appearance.font_family).unwrap().is_empty());
}

#[test]
fn t1786_font_fifo_is_rejected_without_waiting_for_a_writer() {
    use std::os::unix::fs::OpenOptionsExt;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("font-fifo");
    assert!(Command::new("mkfifo").arg(&path).status().unwrap().success());
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader_path = path.clone();
    let reader = std::thread::spawn(move || { sender.send(read_font(&reader_path)).unwrap(); });
    let timely = receiver.recv_timeout(std::time::Duration::from_millis(100));
    if timely.is_err() {
        // Release the deliberate faulty blocking-open implementation before failing.
        let writer = std::fs::OpenOptions::new().write(true).custom_flags(libc::O_NONBLOCK).open(&path).unwrap();
        drop(writer);
    }
    reader.join().unwrap();
    assert!(timely.is_ok_and(|result| result.is_err()), "T1786 blocked on nonregular font input");
}
