//! T1144: native SOCK_SEQPACKET messages expose actual write boundaries.
use super::*;
use crate::camera::output_writer::Writer;
use std::os::{
    fd::{FromRawFd, OwnedFd},
    unix::net::UnixDatagram,
};

fn endpoints() -> (File, UnixDatagram) {
    let mut descriptors = [-1; 2];
    assert_eq!(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                0,
                descriptors.as_mut_ptr(),
            )
        },
        0
    );
    // Successful socketpair creates exactly two independent owned descriptors.
    let output = unsafe { OwnedFd::from_raw_fd(descriptors[0]) };
    let input = unsafe { OwnedFd::from_raw_fd(descriptors[1]) };
    let file = File::from(output);
    socket2::SockRef::from(&file)
        .set_write_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let receiver = UnixDatagram::from(input);
    receiver
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    (file, receiver)
}

fn frames() -> Vec<Vec<u8>> {
    (0..2)
        .map(|frame| {
            (0..115200)
                .map(|index| ((index + frame * 97) % 251) as u8)
                .collect()
        })
        .collect()
}

#[test]
fn t1144_each_full_raw_frame_is_one_native_write_with_identical_bytes() {
    let (output, receiver) = endpoints();
    let prepared = configure_with(&Control::default(), output, spec()).unwrap();
    let mut writer = Writer::new(prepared);
    let expected = frames();
    let mut actual = Vec::new();
    let mut bytes = vec![0; 115201];
    for frame in &expected {
        writer.write(frame).unwrap();
        let count = receiver.recv(&mut bytes).unwrap();
        actual.push(bytes[..count].to_vec());
    }
    drop(writer);
    assert_eq!(receiver.recv(&mut bytes).unwrap(), 0, "T1144 native EOF");
    assert_frames(&actual, &expected);
}

fn assert_frames(actual: &[Vec<u8>], expected: &[Vec<u8>]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "T1144 native frame-write count"
    );
    for (number, (packet, wanted)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            packet.len(),
            wanted.len(),
            "T1144 frame{number} write length"
        );
        let mismatch = packet
            .iter()
            .zip(wanted)
            .position(|(got, value)| got != value);
        assert_eq!(mismatch, None, "T1144 frame{number} first changed byte");
    }
}

#[test]
fn t1144_packet_oracle_rejects_the_observed_fragmented_write_control() {
    use std::io::Write;
    let (mut output, receiver) = endpoints();
    let expected = frames();
    let mut actual = Vec::new();
    let mut bytes = vec![0; 115201];
    // Deliberately reproduce the stock FFmpeg draft's 32768-byte AVIO splitting.
    for frame in &expected {
        for fragment in frame.chunks(32768) {
            assert_eq!(output.write(fragment).unwrap(), fragment.len());
            let count = receiver.recv(&mut bytes).unwrap();
            actual.push(bytes[..count].to_vec());
        }
    }
    drop(output);
    assert_eq!(receiver.recv(&mut bytes).unwrap(), 0);
    assert_eq!(actual.len(), 8);
    assert!(std::panic::catch_unwind(|| assert_frames(&actual, &expected)).is_err());
}
