//! T871 authenticated per-user packet IPC and pinned caller endpoint capabilities.
//! The broker owner supplies a socket path inside its private runtime directory;
//! only that owner may clean stale paths after exclusive ownership election.
use blent_config::connection::registration::{Message, MAX_PACKET};
use blent_config::linux::packet;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::{io, path::Path};
pub mod async_io;
pub mod client;
mod launch;
mod lifetime;
pub mod presentation;
pub mod registration_client;
pub mod display_registration;
mod endpoints;
pub mod entry;
pub mod inventory;
pub mod owner;
pub mod peer;
pub mod server;
pub mod state;
pub use endpoints::Endpoints;

pub struct Socket(OwnedFd);
pub struct Listener(OwnedFd);
pub struct Received {
    pub message: Message,
    pub descriptors: Vec<OwnedFd>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caller {
    pub pid: u32,
    pub uid: u32,
}
fn socket() -> io::Result<OwnedFd> {
    unsafe {
        packet::owned({
            libc::socket(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                0,
            )
        })
    }
}
fn address(path: &Path) -> io::Result<(libc::sockaddr_un, libc::socklen_t)> {
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    let bytes = path.as_os_str().as_bytes();
    if !path.is_absolute() || bytes.contains(&0) || bytes.len() >= address.sun_path.len() {
        return Err(packet::invalid("invalid private broker socket path"));
    }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (out, value) in address.sun_path.iter_mut().zip(bytes) {
        *out = *value as libc::c_char;
    }
    let length =
        (std::mem::offset_of!(libc::sockaddr_un, sun_path) + bytes.len() + 1) as libc::socklen_t;
    Ok((address, length))
}
impl Listener {
    pub fn bind(path: &Path, backlog: u32) -> io::Result<Self> {
        if !(1..=1024).contains(&backlog) {
            return Err(packet::invalid("invalid broker backlog"));
        }
        let (address, length) = address(path)?;
        let fd = socket()?;
        let result = unsafe {
            libc::bind(
                fd.as_raw_fd(),
                (&address as *const libc::sockaddr_un).cast(),
                length,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::listen(fd.as_raw_fd(), backlog as i32) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(fd))
    }
    pub fn accept(&self, expected_uid: u32) -> io::Result<Option<(Socket, Caller)>> {
        let result = unsafe {
            libc::accept4(
                self.as_raw_fd(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            )
        };
        let fd = match unsafe { packet::owned(result) } {
            Ok(fd) => fd,
            Err(error) if packet::retryable(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        let socket = Socket(fd);
        let caller = socket.caller(expected_uid)?;
        Ok(Some((socket, caller)))
    }
}
impl Socket {
    pub fn connect(path: &Path, expected_uid: u32) -> io::Result<Self> {
        let (address, length) = address(path)?;
        let socket = Self(socket()?);
        let result = unsafe {
            libc::connect(
                socket.as_raw_fd(),
                (&address as *const libc::sockaddr_un).cast(),
                length,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        socket.caller(expected_uid)?;
        Ok(socket)
    }
    pub fn caller(&self, expected_uid: u32) -> io::Result<Caller> {
        let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
        let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        let result = unsafe {
            libc::getsockopt(
                self.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut credentials as *mut libc::ucred).cast(),
                &mut length,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        if credentials.uid != expected_uid || credentials.pid <= 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "broker IPC peer is not the selected user",
            ));
        }
        Ok(Caller {
            pid: credentials.pid as u32,
            uid: credentials.uid,
        })
    }
    pub fn send_message(&self, message: &Message, descriptors: &[RawFd]) -> io::Result<()> {
        if descriptors.len() != message.descriptor_count() {
            return Err(packet::invalid("wrong local message descriptor count"));
        }
        let bytes = message
            .encode()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        self.send(&bytes, descriptors)
    }
    pub fn receive_message(&self) -> io::Result<Option<Received>> {
        let mut bytes = [0; MAX_PACKET];
        let Some(packet) = self.receive(&mut bytes)? else {
            return Ok(None);
        };
        let message = Message::decode(&bytes[..packet.length])
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if packet.descriptors.len() != message.descriptor_count() {
            return Err(packet::invalid("wrong local message descriptor count"));
        }
        Ok(Some(Received {
            message,
            descriptors: packet.descriptors,
        }))
    }
    fn send(&self, bytes: &[u8], descriptors: &[RawFd]) -> io::Result<()> {
        packet::send(self.as_raw_fd(), bytes, descriptors)
    }
    fn receive(&self, bytes: &mut [u8]) -> io::Result<Option<packet::Packet>> {
        packet::receive(self.as_raw_fd(), bytes)
    }
}
impl AsRawFd for Socket {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}
impl AsRawFd for Listener {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod control_tests;
