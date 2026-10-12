//! One readiness owner per local packet socket; no per-packet tasks or copies of
//! endpoint descriptors. Cancellation leaves unsent packet ownership with caller.
use super::{owner::Owner, Caller, Received, Socket};
use blent_config::connection::registration::Message;
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::unix::AsyncFd;

pub struct AsyncListener(AsyncFd<Owner>);
/// Supports one receiving owner and one sending owner concurrently. The caller
/// serializes writers to preserve request ordering; no mutex spans native I/O.
pub struct AsyncSocket {
    socket: AsyncFd<Socket>,
    closed: AtomicBool,
}

impl AsyncListener {
    pub fn new(owner: Owner) -> io::Result<Self> {
        Ok(Self(AsyncFd::new(owner)?))
    }

    pub async fn accept(&self) -> io::Result<(AsyncSocket, Caller)> {
        loop {
            let mut readiness = self.0.readable().await?;
            let result = readiness.try_io(|owner| {
                owner
                    .get_ref()
                    .accept(unsafe { libc::geteuid() })?
                    .ok_or_else(would_block)
            });
            if let Ok(result) = result {
                let (socket, caller) = result?;
                return Ok((AsyncSocket::new(socket)?, caller));
            }
        }
    }
}

impl AsyncSocket {
    pub fn new(socket: Socket) -> io::Result<Self> {
        socket.caller(unsafe { libc::geteuid() })?;
        Ok(Self {
            socket: AsyncFd::new(socket)?,
            closed: AtomicBool::new(false),
        })
    }

    pub(super) fn original_lifetime(&self) -> io::Result<super::lifetime::Original> {
        super::lifetime::Original::pin(self.socket()?.get_ref().as_raw_fd())
    }

    fn socket(&self) -> io::Result<&AsyncFd<Socket>> {
        if self.closed.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "broker client socket closed",
            ));
        }
        Ok(&self.socket)
    }

    /// Caller-owned descriptors stay borrowed until an atomic packet send
    /// succeeds. Failed or cancelled sends do not transfer their ownership.
    pub async fn send(&self, message: &Message, descriptors: &[BorrowedFd<'_>]) -> io::Result<()> {
        let result = self.send_packet(message, descriptors).await;
        self.close_on_error(result)
    }

    async fn send_packet(
        &self,
        message: &Message,
        descriptors: &[BorrowedFd<'_>],
    ) -> io::Result<()> {
        if descriptors.len() != message.descriptor_count() {
            return Err(super::packet::invalid("wrong local descriptor count"));
        }
        let bytes = message
            .encode()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let mut raw = [0; 2];
        for (slot, descriptor) in raw.iter_mut().zip(descriptors) {
            *slot = descriptor.as_raw_fd();
        }
        loop {
            let mut readiness = self.socket()?.writable().await?;
            let result =
                readiness.try_io(|socket| socket.get_ref().send(&bytes, &raw[..descriptors.len()]));
            if let Ok(result) = result {
                return result;
            }
        }
    }

    pub async fn receive(&self) -> io::Result<Received> {
        let result = self.receive_packet().await;
        self.close_on_error(result)
    }

    async fn receive_packet(&self) -> io::Result<Received> {
        loop {
            let mut readiness = self.socket()?.readable().await?;
            let result = readiness
                .try_io(|socket| socket.get_ref().receive_message()?.ok_or_else(would_block));
            if let Ok(result) = result {
                return result;
            }
        }
    }

    fn close_on_error<T>(&self, result: io::Result<T>) -> io::Result<T> {
        if result.is_err() {
            self.close();
        }
        if result.is_ok() {
            self.socket()?;
        }
        result
    }

    /// Revoke I/O and wake pending reads/writes. The descriptor stays owned
    /// until this object and every outstanding borrowed operation are dropped.
    pub fn close(&self) {
        if !self.closed.swap(true, Ordering::AcqRel) {
            unsafe {
                libc::shutdown(self.socket.get_ref().as_raw_fd(), libc::SHUT_RDWR);
            }
        }
    }
}

fn would_block() -> io::Error {
    io::Error::from(io::ErrorKind::WouldBlock)
}

#[cfg(test)]
#[path = "async_io_tests.rs"]
mod tests;
