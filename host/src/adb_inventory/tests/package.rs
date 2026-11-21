//! T1653: current package replies never infer absence from an empty success.
use super::*;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

fn output(code: u32, bytes: &[u8], stderr: &[u8]) -> std::process::Output {
    let shift = if cfg!(unix) { 8 } else { 0 };
    std::process::Output {
        status: std::process::ExitStatus::from_raw((code << shift) as _),
        stdout: bytes.to_vec(),
        stderr: stderr.to_vec(),
    }
}

#[test]
fn t1653_empty_success_is_unconfirmed_instead_of_absent() {
    for text in [b"".as_slice(), b"\n", b" \t\r\n"] {
        assert_eq!(package_presence(&output(0, text, b"")), None, "T1653");
        assert_eq!(package_presence(&output(1, text, b"")), Some(false));
    }
}

#[test]
fn t1653_current_package_replies_and_invalid_bytes_remain_distinct() {
    for code in 0..=255 {
        let present = output(code, b"package:/base.apk\npackage:/split.apk\n", b"");
        assert_eq!(package_presence(&present), (code == 0).then_some(true));
        assert_eq!(
            package_presence(&output(code, b"", b"")),
            (code == 1).then_some(false)
        );
        assert_eq!(
            package_presence(&output(code, b"package:/base.apk", b"offline")),
            None
        );
        assert_eq!(package_presence(&output(code, &[255], b"")), None);
        assert_eq!(package_presence(&output(code, b"partial", b"")), None);
    }
}
