#![cfg(feature = "platform")]

use blent_config::edid::Mode;
use std::{path::Path, process::Command};

fn invoke(directory: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_blent-edid"))
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn t1718_cli_uses_shared_encoder_and_independent_fields() {
    let directory = tempfile::tempdir().unwrap();
    let result = invoke(
        directory.path(),
        &[
            "1920",
            "1080",
            "60",
            "display.bin",
            "310",
            "194",
            "--instance",
            "7",
            "--name",
            "Panelé",
        ],
    );
    assert!(result.status.success(), "{:?}", result);
    let block = std::fs::read(directory.path().join("display.bin")).unwrap();
    let mode = Mode {
        width: 1920,
        height: 1080,
        refresh: 60,
        width_mm: 310,
        height_mm: 194,
        instance: 7,
    };
    assert_eq!(block, mode.encode("Panelé").unwrap());
    assert_eq!(block.iter().map(|&b| u32::from(b)).sum::<u32>() % 256, 0);
    assert_eq!(&block[12..16], &8u32.to_le_bytes());
    assert_eq!(&block[95..108], b"Panel?\n      ");
    assert_eq!(
        u16::from(block[56]) | (u16::from(block[58] >> 4) << 8),
        1920
    );
    assert_eq!(
        u16::from(block[59]) | (u16::from(block[61] >> 4) << 8),
        1080
    );
}

#[test]
fn t1718_invalid_requests_preserve_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("display.bin");
    std::fs::write(&output, b"previous owned output").unwrap();
    for args in [
        vec!["640", "480", "10", "display.bin"],
        vec!["640", "480", "25", "display.bin", "0", "194"],
        vec!["640", "480", "25", "display.bin", "310", "0"],
        vec!["640", "480", "25", "display.bin", "4096", "194"],
        vec!["640", "480", "25", "display.bin", "310", "4096"],
        vec![
            "640",
            "480",
            "25",
            "display.bin",
            "--instance",
            "4294967295",
        ],
        vec!["4294967296", "480", "25", "display.bin"],
        vec!["invalid", "480", "25", "display.bin"],
        vec!["640", "480", "25", "display.bin", "--unknown"],
    ] {
        let result = invoke(directory.path(), &args);
        assert!(
            matches!(result.status.code(), Some(1 | 2)),
            "{:?}: {:?}",
            args,
            result
        );
        assert_eq!(
            std::fs::read(&output).unwrap(),
            b"previous owned output",
            "{:?}",
            args
        );
    }
}

#[test]
fn t1718_output_errors_create_no_files() {
    let directory = tempfile::tempdir().unwrap();
    for output in [".", "missing/display.bin"] {
        let result = invoke(directory.path(), &["640", "480", "25", output]);
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&result.stderr).contains("write EDID output"));
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[test]
fn t1718_defaults_and_help_are_explicit() {
    let directory = tempfile::tempdir().unwrap();
    assert!(invoke(directory.path(), &["--help"]).status.success());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    assert!(invoke(directory.path(), &[]).status.success());
    let block = std::fs::read(directory.path().join("s9ultra.bin")).unwrap();
    let mode = Mode {
        width: 2960,
        height: 1848,
        refresh: 60,
        width_mm: 310,
        height_mm: 194,
        instance: 0,
    };
    assert_eq!(block, mode.encode("Blent").unwrap());
}
