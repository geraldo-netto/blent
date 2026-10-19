use super::*;
use std::path::{Path, PathBuf};
pub(super) fn compile(root: &Path) -> PathBuf {
    let executable = root.join("v4l2-oracle");
    let status = std::process::Command::new("cc")
        .args(["-std=c11", "-D_GNU_SOURCE", "-Wall", "-Wextra", "-Werror"])
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/camera/output_device/tests/abi.c"
        ))
        .arg("-o")
        .arg(&executable)
        .status()
        .unwrap();
    assert!(status.success());
    executable
}
pub(super) fn numbers(output: std::process::Output) -> Vec<u64> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .map(|word| word.parse().unwrap())
        .collect()
}
#[test]
fn t1143_uapi_matches_installed_independent_c_headers() {
    use std::mem::{align_of, offset_of, size_of};
    let root = tempfile::tempdir().unwrap();
    let executable = compile(root.path());
    let actual = numbers(std::process::Command::new(executable).output().unwrap());
    let expected = [
        size_of::<abi::Format>() as u64,
        align_of::<abi::Format>() as u64,
        offset_of!(abi::Format, image) as u64,
        size_of::<abi::Pixels>() as u64,
        offset_of!(abi::Pixels, bytes) as u64,
        size_of::<abi::Parameters>() as u64,
        align_of::<abi::Parameters>() as u64,
        offset_of!(abi::Parameters, timing) as u64,
        size_of::<abi::Rate>() as u64,
        size_of::<abi::Fraction>() as u64,
        u64::from(linux_raw_sys::ioctl::VIDIOC_S_FMT),
        u64::from(linux_raw_sys::ioctl::VIDIOC_G_FMT),
        u64::from(linux_raw_sys::ioctl::VIDIOC_G_PARM),
        u64::from(linux_raw_sys::ioctl::VIDIOC_S_PARM),
        u64::from(abi::YUV420),
        1,
        2,
        0x1000,
    ];
    assert_eq!(actual, expected);
}

fn boundary_source(request: &str) -> String {
    format!(
        r#"extern crate self as libc;
extern crate self as linux_raw_sys;
pub use core::ffi::{{c_int, c_ulong}};
unsafe extern "C" {{ pub fn ioctl(fd: c_int, request: {request}, ...) -> c_int; }}
pub mod ioctl {{
    pub const VIDIOC_S_FMT: u32 = {set_format};
    pub const VIDIOC_G_PARM: u32 = {get_rate};
    pub const VIDIOC_S_PARM: u32 = {set_rate};
}}
pub mod boundary {{
    pub mod abi {{
        #[repr(C)] pub struct Format {{ pub opaque: u32 }}
        #[repr(C)] pub struct Parameters {{ pub opaque: u32 }}
    }}
    #[path = {source:?}] mod native;
    pub fn exercise(fd: std::os::fd::BorrowedFd<'_>, format: &mut abi::Format, rate: &mut abi::Parameters) {{
        use native::Controls;
        let _ = native::Ioctl.set_format(fd, format);
        let _ = native::Ioctl.get_rate(fd, rate);
        let _ = native::Ioctl.set_rate(fd, rate);
    }}
}}
"#,
        set_format = linux_raw_sys::ioctl::VIDIOC_S_FMT,
        get_rate = linux_raw_sys::ioctl::VIDIOC_G_PARM,
        set_rate = linux_raw_sys::ioctl::VIDIOC_S_PARM,
        source = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/camera/output_device/native.rs"
        ),
    )
}

#[test]
fn t1143_ioctl_boundary_accepts_signed_and_unsigned_native_request_widths() {
    let root = tempfile::tempdir().unwrap();
    for request in ["c_ulong", "c_int", "u32"] {
        let source = root.path().join("boundary.rs");
        std::fs::write(&source, boundary_source(request)).unwrap();
        let output = std::process::Command::new("rustc")
            .args(["--edition=2021", "--crate-type=lib", "-Dwarnings"])
            .arg(&source)
            .arg("-o")
            .arg(root.path().join("boundary.rlib"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "T1143 {request}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
