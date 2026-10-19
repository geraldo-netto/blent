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

#[test]
fn t1144_virtual_frame_transfer_fixture() {
    let Some(path) = std::env::var_os("BLENT_T1143_DEVICE") else {
        return;
    };
    for fps in [30, 5, 7, 30] {
        transfer(&path, fps);
    }
}

fn transfer(path: &std::ffi::OsStr, fps: u32) {
    use crate::camera::output_writer::Writer;
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    let prepared =
        configure_blocking(file, Spec::from_profile(&profile(320, 240, fps)).unwrap()).unwrap();
    let expected: Vec<u8> = (0..115200)
        .map(|index| ((index * 17 + fps as usize) % 251) as u8)
        .collect();
    let mut writer = Writer::new(prepared);
    // The driver becomes a readable capture source only after an output frame.
    writer.write(&expected).unwrap();
    let mut reader = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .unwrap();
    let mut received = vec![0; 115200];
    let initial = reader.read(&mut received);
    assert!(
        initial.as_ref().is_ok_and(|&count| count == expected.len())
            || initial
                .as_ref()
                .is_err_and(|error| error.kind() == io::ErrorKind::WouldBlock),
        "T1144 initial capture: {initial:?}"
    );
    writer.write(&expected).unwrap();
    let mut polling = libc::pollfd {
        fd: reader.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    assert_eq!(
        unsafe { libc::poll(&mut polling, 1, 5000) },
        1,
        "T1144 capture readiness"
    );
    assert_eq!(reader.read(&mut received).unwrap(), expected.len());
    let mismatch = received
        .iter()
        .zip(&expected)
        .position(|(got, wanted)| got != wanted);
    assert_eq!(
        mismatch, None,
        "T1144 captured first changed byte at {fps} FPS"
    );
    println!("T1144 real driver captured identical115200-byte frame at {fps} FPS");
}
