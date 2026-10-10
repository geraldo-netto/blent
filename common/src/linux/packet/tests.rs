use super::*;
use std::os::fd::{AsRawFd, OwnedFd};
fn pair() -> (OwnedFd, OwnedFd) {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        },
        0
    );
    unsafe { (owned(fds[0]).unwrap(), owned(fds[1]).unwrap()) }
}
#[test]
fn t871_packets_preserve_boundaries_and_owned_cloexec_rights() {
    let (left, right) = pair();
    let file = tempfile::tempfile().unwrap();
    assert!(receive(right.as_raw_fd(), &mut [0; 4]).unwrap().is_none());
    send(left.as_raw_fd(), b"test", &[file.as_raw_fd()]).unwrap();
    let mut bytes = [0; MAX_BYTES];
    let packet = receive(right.as_raw_fd(), &mut bytes).unwrap().unwrap();
    assert_eq!(&bytes[..packet.length], b"test");
    assert_eq!(packet.descriptors.len(), 1);
    assert_ne!(
        unsafe { libc::fcntl(packet.descriptors[0].as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
        0
    );
    send(left.as_raw_fd(), b"next", &[]).unwrap();
    assert!(receive(right.as_raw_fd(), &mut bytes)
        .unwrap()
        .unwrap()
        .descriptors
        .is_empty());
    drop(left);
    assert!(receive(right.as_raw_fd(), &mut bytes).is_err());
}
#[test]
fn t871_packet_parameter_bounds_and_native_errors_are_rejected() {
    let (left, right) = pair();
    assert!(receive(right.as_raw_fd(), &mut []).is_err());
    for size in [MAX_BYTES + 1, MAX_BYTES * 2] {
        assert!(send(left.as_raw_fd(), &vec![0; size], &[]).is_err());
        assert!(receive(right.as_raw_fd(), &mut vec![0; size]).is_err());
    }
    assert!(send(left.as_raw_fd(), b"x", &[-1]).is_err());
    assert!(send(left.as_raw_fd(), b"x", &[left.as_raw_fd(); MAX_FDS + 1]).is_err());
    assert!(send(-1, b"x", &[]).is_err());
    assert!(receive(-1, &mut [0; 1]).is_err());
    assert!(unsafe { owned(-1) }.is_err());
    for error in [libc::EINTR, libc::EAGAIN, libc::EIO] {
        assert_eq!(
            retryable(&io::Error::from_raw_os_error(error)),
            error != libc::EIO
        );
    }
}
#[test]
fn t871_payload_truncation_and_unknown_control_release_received_rights() {
    let (left, right) = pair();
    let file = tempfile::tempfile().unwrap();
    let before = aliases(file.as_raw_fd());
    send(left.as_raw_fd(), b"oversized", &[file.as_raw_fd()]).unwrap();
    assert!(receive(right.as_raw_fd(), &mut [0; 1]).is_err());
    assert_eq!(aliases(file.as_raw_fd()), before);
    let enabled = 1i32;
    assert_eq!(
        unsafe {
            libc::setsockopt(
                right.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PASSCRED,
                (&enabled as *const i32).cast(),
                4,
            )
        },
        0
    );
    send(left.as_raw_fd(), b"x", &[file.as_raw_fd()]).unwrap();
    assert!(receive(right.as_raw_fd(), &mut [0; 1]).is_err());
    assert_eq!(aliases(file.as_raw_fd()), before);
}
fn aliases(fd: i32) -> usize {
    use std::os::unix::fs::MetadataExt;
    let target = std::fs::metadata(format!("/proc/self/fd/{fd}")).unwrap();
    std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(|entry| std::fs::metadata(entry.ok()?.path()).ok())
        .filter(|metadata| (metadata.dev(), metadata.ino()) == (target.dev(), target.ino()))
        .count()
}
#[test]
fn t871_full_packet_and_maximum_rights_are_bounded() {
    let (left, right) = pair();
    let file = tempfile::tempfile().unwrap();
    send(
        left.as_raw_fd(),
        &vec![1; MAX_BYTES],
        &[file.as_raw_fd(); MAX_FDS],
    )
    .unwrap();
    let mut bytes = vec![0; MAX_BYTES];
    let packet = receive(right.as_raw_fd(), &mut bytes).unwrap().unwrap();
    assert_eq!(packet.length, MAX_BYTES);
    assert_eq!(packet.descriptors.len(), MAX_FDS);
    assert!(bytes.iter().all(|b| *b == 1));
}

#[test]
fn t871_excess_and_truncated_rights_close_every_delivered_descriptor() {
    for credentials in [false, true] {
        for count in [MAX_FDS + 1, 32, 64] {
            let (left, right) = pair();
            let file = tempfile::tempfile().unwrap();
            let before = aliases(file.as_raw_fd());
            pass_credentials(right.as_raw_fd(), credentials);
            raw_rights(left.as_raw_fd(), file.as_raw_fd(), count);
            assert!(receive(right.as_raw_fd(), &mut [0; 1]).is_err());
            assert_eq!(
                aliases(file.as_raw_fd()),
                before,
                "T871 leaked excess/truncated rights: credentials={credentials}, count={count}"
            );
            assert!(receive(right.as_raw_fd(), &mut [0; 1]).unwrap().is_none());
        }
    }
}

#[test]
fn t871_empty_payload_owns_and_closes_rights_before_rejection() {
    let (left, right) = pair();
    let file = tempfile::tempfile().unwrap();
    let before = aliases(file.as_raw_fd());
    send(left.as_raw_fd(), &[], &[file.as_raw_fd()]).unwrap();
    assert!(receive(right.as_raw_fd(), &mut [0; 1]).is_err());
    assert_eq!(aliases(file.as_raw_fd()), before);
    assert!(receive(right.as_raw_fd(), &mut [0; 1]).unwrap().is_none());
}

#[test]
fn t871_bounded_invalid_sizes_and_descriptor_counts_never_reach_syscall() {
    for offset in 1..=256 {
        let bytes = vec![0; MAX_BYTES + offset];
        assert_eq!(
            send(-1, &bytes, &[]).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let mut bytes = bytes;
        assert_eq!(
            receive(-1, &mut bytes).err().unwrap().kind(),
            io::ErrorKind::InvalidData
        );
        let descriptors = vec![-1; MAX_FDS + offset];
        assert_eq!(
            send(-1, b"x", &descriptors).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}

fn pass_credentials(socket: i32, enabled: bool) {
    let value = i32::from(enabled);
    assert_eq!(
        unsafe {
            libc::setsockopt(
                socket,
                libc::SOL_SOCKET,
                libc::SO_PASSCRED,
                (&value as *const i32).cast(),
                std::mem::size_of::<i32>() as libc::socklen_t,
            )
        },
        0
    );
}

fn raw_rights(socket: i32, fd: i32, count: usize) {
    assert!((1..=64).contains(&count));
    let bytes = b"x";
    let mut vector = libc::iovec {
        iov_base: bytes.as_ptr().cast_mut().cast(),
        iov_len: 1,
    };
    let mut buffer = [0usize; 64];
    let mut header: libc::msghdr = unsafe { std::mem::zeroed() };
    header.msg_iov = &mut vector;
    header.msg_iovlen = 1;
    header.msg_control = buffer.as_mut_ptr().cast();
    let length = (count * std::mem::size_of::<i32>()) as u32;
    header.msg_controllen = unsafe { libc::CMSG_SPACE(length) } as usize;
    unsafe {
        let control = libc::CMSG_FIRSTHDR(&header);
        (*control).cmsg_level = libc::SOL_SOCKET;
        (*control).cmsg_type = libc::SCM_RIGHTS;
        (*control).cmsg_len = libc::CMSG_LEN(length) as usize;
        for index in 0..count {
            std::ptr::write_unaligned(libc::CMSG_DATA(control).cast::<i32>().add(index), fd);
        }
        assert_eq!(libc::sendmsg(socket, &header, libc::MSG_NOSIGNAL), 1);
    }
}
