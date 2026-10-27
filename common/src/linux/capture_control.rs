//! Linux capture control. Cancellation retains the original pending transition.
use crate::capture_activity::{Request, RECORD_BYTES};
use crate::linux::packet;
use anyhow::{ensure, Result};
use std::io;
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::Arc;
use tokio::io::unix::AsyncFd;

struct Pending {
    request: Request,
    sent: bool,
}

pub struct Control {
    socket: Arc<AsyncFd<OwnedFd>>,
    completed: Option<Request>,
    pending: Option<Pending>,
    failed: bool,
}

pub struct Inherited(OwnedFd);

impl Inherited {
    /// Reserve a child-owned descriptor above stdio before fork. Only this
    /// endpoint crosses exec; unrelated descriptors and the error pipe survive.
    pub fn attach(self, command: &mut tokio::process::Command) -> io::Result<()> {
        let fd = unsafe { libc::fcntl(self.0.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 3) };
        let child = unsafe { packet::owned(fd) }?;
        command.arg("--capture-activity-fd").arg(fd.to_string());
        unsafe {
            command.pre_exec(move || inherit(child.as_raw_fd()));
        }
        Ok(())
    }
}

fn inherit(child: i32) -> io::Result<()> {
    if unsafe { libc::fcntl(child, libc::F_SETFD, 0) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

impl Control {
    pub fn pair() -> io::Result<(Self, Inherited)> {
        let mut pair = [-1; 2];
        let result = unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
                pair.as_mut_ptr(),
            )
        };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        let parent = unsafe { packet::owned(pair[0]) }?;
        let child = unsafe { packet::owned(pair[1]) }?;
        Ok((
            Self {
                socket: Arc::new(AsyncFd::new(parent)?),
                completed: None,
                pending: None,
                failed: false,
            },
            Inherited(child),
        ))
    }

    /// Retain with the original native process until its actual retirement.
    /// This resource lease carries no authority to issue commands.
    pub fn lease(&self) -> impl Send + Sync + 'static {
        self.socket.clone()
    }

    /// Never replaces an unfinished command, including one whose caller was
    /// canceled. The exact echo certifies only this native capture boundary;
    /// encoder/process retirement requires its own independent certificate.
    pub async fn set_active(&mut self, active: bool) -> Result<Request> {
        ensure!(!self.failed, "capture control failed");
        let result = self.transition(active).await;
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    async fn transition(&mut self, active: bool) -> Result<Request> {
        self.finish().await?;
        if let Some(request) = self.completed.filter(|request| request.active() == active) {
            return Ok(request);
        }
        let sequence = self.completed.map_or(0, Request::sequence);
        let request = Request::after(sequence, active)?;
        self.pending = Some(Pending {
            request,
            sent: false,
        });
        self.finish().await?;
        Ok(request)
    }

    async fn finish(&mut self) -> Result<()> {
        let Some(pending) = self.pending.as_ref() else {
            return Ok(());
        };
        let request = pending.request;
        if !pending.sent {
            self.send(request).await?;
            self.pending.as_mut().unwrap().sent = true;
        }
        ensure!(
            self.receive().await? == request,
            "foreign capture acknowledgment"
        );
        self.completed = Some(request);
        self.pending = None;
        Ok(())
    }

    async fn send(&self, request: Request) -> io::Result<()> {
        loop {
            let mut ready = self.socket.writable().await?;
            let result = ready.try_io(|socket| {
                packet::send(socket.get_ref().as_raw_fd(), &request.encode(), &[])
            });
            if let Ok(result) = result {
                match result {
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    result => return result,
                }
            }
        }
    }

    async fn receive(&self) -> Result<Request> {
        loop {
            let mut ready = self.socket.readable().await?;
            let result = ready.try_io(|socket| receive_record(socket.get_ref()));
            if let Ok(result) = result {
                return Request::decode(&result?);
            }
        }
    }
}

fn receive_record(socket: &OwnedFd) -> io::Result<[u8; RECORD_BYTES]> {
    let mut bytes = [0; RECORD_BYTES];
    let received = packet::receive(socket.as_raw_fd(), &mut bytes)?
        .ok_or_else(|| io::Error::from(io::ErrorKind::WouldBlock))?;
    if received.length != RECORD_BYTES || !received.descriptors.is_empty() {
        return Err(packet::invalid("invalid capture acknowledgment packet"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
