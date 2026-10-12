//! Kernel proof tied to the original socket peer, without a PID lookup/reuse race.
use blent_config::{connection::registration_client::BrokerLifetime, linux::packet};
use std::{
    io,
    os::fd::{AsRawFd, OwnedFd, RawFd},
};

pub(super) struct Original(OwnedFd);
impl Original {
    pub(super) fn pin(socket: RawFd) -> io::Result<Self> {
        let mut fd: libc::c_int = -1;
        let mut length = std::mem::size_of_val(&fd) as libc::socklen_t;
        let result = unsafe {
            libc::getsockopt(
                socket,
                libc::SOL_SOCKET,
                libc::SO_PEERPIDFD,
                (&mut fd as *mut libc::c_int).cast(),
                &mut length,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        let owned = unsafe { packet::owned(fd)? };
        if length != std::mem::size_of_val(&fd) as libc::socklen_t {
            return Err(packet::invalid("invalid broker process observer"));
        }
        Ok(Self(owned))
    }
}
impl BrokerLifetime for Original {
    fn exited(&self) -> bool {
        let mut descriptor = libc::pollfd {
            fd: self.0.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // Never block the shared retirement reaper. Errors/invalid descriptors
        // are not proof; readiness on a pidfd proves this exact process exited.
        unsafe { libc::poll(&mut descriptor, 1, 0) == 1 && descriptor.revents & libc::POLLIN != 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1007_live_socket_peer_is_pinned_with_close_on_exec() {
        let (socket, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
        let original = Original::pin(socket.as_raw_fd()).unwrap();
        assert!(!original.exited());
        let flags = unsafe { libc::fcntl(original.0.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0 && flags & libc::FD_CLOEXEC != 0);
        drop(socket);
        assert!(!original.exited()); // Closing IPC is not process exit.
    }
    #[test]
    fn t1007_invalid_and_non_socket_inputs_cannot_establish_exit_proof() {
        for fd in [-1, -2, i32::MIN, i32::MAX] {
            assert!(Original::pin(fd).is_err());
        }
        let file = std::fs::File::open("/dev/null").unwrap();
        assert!(Original::pin(file.as_raw_fd()).is_err());
    }
}
