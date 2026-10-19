//! Re-executed only by the isolated guest runner; ordinary rejection/property
//! tests do not interpret this fixture entry point as successful driver evidence.
use super::*;
use std::{
    io::Write,
    os::fd::AsRawFd,
    process::{Command, Stdio},
};

#[test]
fn t1143_virtual_output_process_fixture() {
    let Some(path) = std::env::var_os("BLENT_T1143_DEVICE") else {
        return;
    };
    let oracle = std::env::var_os("BLENT_T1143_ORACLE").unwrap();
    for fps in [30, 5, 7, 30] {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        assert_eq!(
            unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        let shape = Spec::from_profile(&profile(320, 240, fps)).unwrap();
        let prepared = configure_blocking(file, shape).unwrap();
        let (mut output, original_spec) = prepared.into_parts();
        assert_eq!(original_spec.frame_bytes(), 115200);
        let observed = super::oracle::numbers(
            Command::new(&oracle)
                .arg("--snapshot")
                .stdin(Stdio::from(output.try_clone().unwrap()))
                .output()
                .unwrap(),
        );
        assert_eq!(
            &observed[..6],
            &[
                320,
                240,
                u64::from(u32::from_le_bytes(*b"YU12")),
                1,
                320,
                115200
            ]
        );
        assert_eq!(observed[6] * u64::from(fps), observed[7]);
        assert_eq!(output.write(&vec![128; 115200]).unwrap(), 115200);
        println!("T1143 configured/wrote/independently queried 320x240@{fps}");
    }
}
