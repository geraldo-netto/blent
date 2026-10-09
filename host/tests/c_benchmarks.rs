#![cfg(target_os = "linux")]
use std::{path::PathBuf, process::Command};

#[test]
fn t764_conversion_uses_actual_padded_stride() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("stride-test");
    let output = Command::new("cc").args([
        "-std=c11", "-O1", "-g", "-pthread", "-fsanitize=address,undefined",
        "-fno-sanitize-recover=all", "-fno-pie", "-no-pie", "-I",
    ]).arg(root.join("evdi")).arg(root.join("tests/benchmark_stride.c"))
        .arg(root.join("evdi/conversion.c")).arg(root.join("evdi/frame_exchange.c"))
        .arg("-o").arg(&binary).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let output = Command::new(binary).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn t761_producer_rejects_invalid_geometry_before_conversion() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_owned();
    let dir = std::env::temp_dir().join(format!("blent-t761-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let binary = dir.join("producer");
    let build = Command::new("cc").current_dir(&root).args([
        "-std=c11", "-O1", "-g", "-pthread", "-fsanitize=address,undefined",
        "-fno-sanitize-recover=all", "-fno-pie", "-no-pie", "-Ihost/evdi",
        "scripts/benchmarks/shared-encode-producer.c", "host/evdi/conversion.c",
        "host/evdi/raw_ring.c", "-o",
    ]).arg(&binary).output().unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let output = Command::new(&binary).args([
        "fifo", "/dev/null", "3", "2", "1", "60", "1", "unused", "-1",
    ]).output().unwrap();
    assert_eq!(output.status.code(), Some(2), "T761 must reject unsafe geometry: {}", String::from_utf8_lossy(&output.stderr));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Sanitizer"));
    let valid = ["fifo", "/dev/null", "2", "2", "1", "60", "1", "unused", "-1"];
    for field in [2, 3, 4, 5, 6, 8] {
        for value in ["", "junk", "2tail", "-2147483649", "2147483648", "999999999999999999999", "-2"] {
            let mut args = valid;
            args[field] = value;
            let output = Command::new(&binary).args(args).output().unwrap();
            assert_eq!(output.status.code(), Some(2), "T761 field {field} value {value}: {}", String::from_utf8_lossy(&output.stderr));
            assert!(!String::from_utf8_lossy(&output.stderr).contains("Sanitizer"));
        }
    }
    for (field, value) in [(2, "0"), (2, "1"), (2, "4097"), (3, "3"), (3, "4097"), (4, "0"), (5, "0"), (5, "241"), (6, "0"), (6, "129"), (0, "unknown")] {
        let mut args = valid;
        args[field] = value;
        assert_eq!(Command::new(&binary).args(args).output().unwrap().status.code(), Some(2));
    }
    assert_eq!(Command::new(&binary).output().unwrap().status.code(), Some(2));
    let output = Command::new(&binary).args(valid).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Sanitizer"));
    std::fs::remove_dir_all(dir).unwrap();
}
