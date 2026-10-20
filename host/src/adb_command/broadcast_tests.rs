//! T1205: private native command fixtures; no ADB or device access.
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub(crate) fn fixture(root: &Path, stdout: &[u8], success: bool) -> PathBuf {
    std::fs::write(root.join("stdout"), stdout).unwrap();
    let path = root.join("adb");
    let code = format!("#!/usr/bin/python3\nimport pathlib, sys\nsys.stdin.buffer.read()\nsys.stdout.buffer.write(pathlib.Path(__file__).with_name('stdout').read_bytes())\nsys.exit({})\n", i32::from(!success));
    std::fs::write(&path, code).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}

pub(crate) fn cases() -> Vec<(Vec<u8>, bool)> {
    let mut cases = vec![
        (b"Broadcast completed: result=10\n".to_vec(), false),
        (b"Broadcast completed: result=1garbage\n".to_vec(), false),
        (b"result=1\n".to_vec(), false),
        (b"prefix Broadcast completed: result=1\n".to_vec(), false),
        (
            b"Broadcast completed: result=1\nBroadcast completed: result=0\n".to_vec(),
            false,
        ),
        (
            b"Broadcast completed: result=1\nBroadcast completed: result=1\n".to_vec(),
            false,
        ),
        (b"\xffBroadcast completed: result=1\n".to_vec(), false),
        (Vec::new(), false),
        (b"Broadcast completed: result=1\n".to_vec(), true),
        (
            b"Broadcasting: Intent { ... }\r\nBroadcast completed: result=1\r\n".to_vec(),
            true,
        ),
    ];
    for value in -16..=32 {
        cases.push((
            format!("Broadcast completed: result={value}\n").into_bytes(),
            value == 1,
        ));
    }
    cases
}

#[test]
fn t1205_bounded_acknowledgements_match_independent_exact_result_oracle() {
    for value in -1024..=1024 {
        let stdout = format!("Broadcast completed: result={value}\n");
        assert_eq!(super::broadcast_accepted(stdout.as_bytes()), value == 1);
    }
    for (stdout, expected) in cases() {
        assert_eq!(super::broadcast_accepted(&stdout), expected, "{stdout:?}");
    }
}

#[test]
fn t1205_invalid_utf8_cannot_grant_an_otherwise_valid_acknowledgement() {
    let accepted = b"Broadcast completed: result=1\n";
    for position in 0..=accepted.len() {
        for invalid in 128..=255 {
            let mut stdout = accepted.to_vec();
            stdout.insert(position, invalid);
            assert!(!super::broadcast_accepted(&stdout), "{position} {invalid}");
        }
    }
}
