//! Bounded Linux sequenced-packet descriptor transport shared by local adapters.
//! Every received descriptor becomes owned before payload/control rejection.
use std::io;
use std::os::fd::{FromRawFd, OwnedFd, RawFd};
mod control;
pub const MAX_BYTES: usize = 4096;
pub const MAX_FDS: usize = 8;
pub struct Packet {
    pub length: usize,
    pub descriptors: Vec<OwnedFd>,
}

/// Adopt a descriptor freshly returned by an ownership-transferring syscall.
/// # Safety
/// Every nonnegative descriptor must be live and exclusively owned by the caller;
/// ownership transfers here even if later validation fails.
/// ```compile_fail
/// let _ = blent_config::linux::packet::owned(3);
/// ```
pub unsafe fn owned(fd: i32) -> io::Result<OwnedFd> {
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}
pub fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
pub fn retryable(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
    )
}
pub fn send(socket: RawFd, bytes: &[u8], descriptors: &[RawFd]) -> io::Result<()> {
    if bytes.len() > MAX_BYTES || descriptors.len() > MAX_FDS {
        return Err(invalid("local packet exceeds bounds"));
    }
    let mut iov = libc::iovec {
        iov_base: bytes.as_ptr().cast_mut().cast(),
        iov_len: bytes.len(),
    };
    let mut header: libc::msghdr = unsafe { std::mem::zeroed() };
    header.msg_iov = &mut iov;
    header.msg_iovlen = 1;
    let mut control = [0usize; 16];
    control::sending(&mut header, &mut control, descriptors);
    let result = unsafe { libc::sendmsg(socket, &header, libc::MSG_NOSIGNAL) };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    if result != bytes.len() as isize {
        return Err(invalid("incomplete local packet write"));
    }
    Ok(())
}
pub fn receive(socket: RawFd, bytes: &mut [u8]) -> io::Result<Option<Packet>> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err(invalid("invalid local packet buffer"));
    }
    let mut iov = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: bytes.len(),
    };
    let mut header: libc::msghdr = unsafe { std::mem::zeroed() };
    let mut control = [0usize; 16];
    header.msg_iov = &mut iov;
    header.msg_iovlen = 1;
    header.msg_control = control.as_mut_ptr().cast();
    header.msg_controllen = std::mem::size_of_val(&control);
    let result = unsafe { libc::recvmsg(socket, &mut header, libc::MSG_CMSG_CLOEXEC) };
    if result < 0 {
        return receive_error();
    }
    let descriptors = control::received(&header)?;
    if result == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "local packet peer closed",
        ));
    }
    if header.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) != 0 {
        return Err(invalid("truncated local packet"));
    }
    Ok(Some(Packet {
        length: result as usize,
        descriptors,
    }))
}
fn receive_error() -> io::Result<Option<Packet>> {
    let error = io::Error::last_os_error();
    if retryable(&error) {
        return Ok(None);
    }
    Err(error)
}

#[cfg(test)]
mod tests;
