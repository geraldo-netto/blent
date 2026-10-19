//! T1144: bounded native/fault inputs, one-attempt semantics and exact OFD lease.
use super::*;
use std::{cell::Cell, os::fd::AsRawFd};

fn spec() -> Spec {
    Spec::from_profile(&blent_config::camera::CameraProfile {
        width: 320,
        height: 240,
        ..Default::default()
    })
    .unwrap()
}
fn unprepared(output: File) -> Writer {
    // Unit fault seam only: no native call is made before the supplied port.
    Writer {
        output,
        spec: spec(),
        failed: false,
    }
}
fn reusable(file: &File) -> bool {
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        assert_eq!(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) }, 0);
        return true;
    }
    assert_eq!(io::Error::last_os_error().kind(), io::ErrorKind::WouldBlock);
    false
}

#[test]
fn t1144_invalid_lengths_have_no_native_effect_and_leave_writer_usable() {
    let mut writer = unprepared(File::open("/dev/null").unwrap());
    let capacity = spec().frame_bytes() as usize;
    let bytes = vec![0; capacity + 1];
    // All short lengths and the first overlong boundary in a finite admitted domain.
    for length in 0..=capacity + 1 {
        if length == capacity {
            continue;
        }
        let result = writer.write_with(&bytes[..length], |_, _| panic!("T1144 invalid effect"));
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }
    assert!(!writer.failed);
    writer
        .write_with(&bytes[..capacity], |_, frame| Ok(frame.len()))
        .unwrap();
}

#[test]
fn t1144_short_zero_oversized_and_native_errors_poison_without_retry() {
    let capacity = spec().frame_bytes() as usize;
    let frame = vec![0; capacity];
    for count in [0, 1, capacity / 2, capacity - 1, capacity + 1, usize::MAX] {
        let mut writer = unprepared(File::open("/dev/null").unwrap());
        let calls = Cell::new(0);
        let result = writer.write_with(&frame, |_, _| {
            calls.set(calls.get() + 1);
            Ok(count)
        });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::WriteZero);
        assert_eq!(calls.get(), 1);
        assert_poisoned(&mut writer, &frame);
    }
    for code in [
        libc::EIO,
        libc::EINTR,
        libc::ENOSPC,
        libc::EBADF,
        libc::EAGAIN,
    ] {
        let mut writer = unprepared(File::open("/dev/null").unwrap());
        let result = writer.write_with(&frame, |_, _| Err(io::Error::from_raw_os_error(code)));
        assert_eq!(result.unwrap_err().raw_os_error(), Some(code));
        assert_poisoned(&mut writer, &frame);
    }
}
fn assert_poisoned(writer: &mut Writer, frame: &[u8]) {
    let result = writer.write_with(frame, |_, _| panic!("T1144 retry after native failure"));
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
}

#[test]
fn t1144_panicking_native_port_cannot_reenter_before_reconfiguration() {
    let mut writer = unprepared(File::open("/dev/null").unwrap());
    let frame = vec![0; spec().frame_bytes() as usize];
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        writer.write_with(&frame, |_, _| panic!("T1144 partial native panic"))
    }))
    .is_err());
    assert_poisoned(&mut writer, &frame);
}

#[test]
fn t1144_original_lease_stays_owned_across_success_rejection_and_failure() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("original");
    let original = File::create(&path).unwrap();
    let probe = File::open(&path).unwrap();
    assert_eq!(
        unsafe { libc::flock(original.as_raw_fd(), libc::LOCK_EX) },
        0
    );
    let mut writer = unprepared(original);
    let frame = vec![0; spec().frame_bytes() as usize];
    assert!(!reusable(&probe));
    writer.write(&frame).unwrap();
    assert!(writer.write(&[]).is_err());
    assert!(!reusable(&probe));
    assert!(writer
        .write_with(&frame, |_, _| Err(io::Error::other("T1144 failure")))
        .is_err());
    assert!(!reusable(&probe));
    assert_poisoned(&mut writer, &frame);
    drop(writer);
    assert!(reusable(&probe));
}

#[test]
fn t1144_real_read_only_file_failure_poisoned_and_released() {
    let mut writer = unprepared(File::open("/dev/null").unwrap());
    let frame = vec![0; spec().frame_bytes() as usize];
    assert_eq!(
        writer.write(&frame).unwrap_err().raw_os_error(),
        Some(libc::EBADF)
    );
    assert_poisoned(&mut writer, &frame);
}
