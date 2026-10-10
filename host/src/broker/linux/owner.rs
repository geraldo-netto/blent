//! Stable per-user election and descriptor-relative endpoint ownership.
//! The selected private directory must remain under the user's control; this is
//! not a sandbox against hostile code already running as that same user.
use super::{Caller, Listener, Socket};
use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

const LOCK: &CStr = c"broker.lock";
const SOCKET: &CStr = c"broker.sock";

pub struct Owner {
    listener: Listener,
    directory: File,
    _lock: File,
    identity: (u64, u64),
}

impl Owner {
    /// Bind fixed broker.sock beneath an existing private directory. The stable
    /// broker.lock inode is never removed, including on ordinary shutdown.
    pub fn bind(path: &Path, backlog: u32) -> io::Result<Self> {
        if !path.is_absolute() || !(1..=1024).contains(&backlog) {
            return Err(invalid("invalid broker directory or backlog"));
        }
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        validate_directory(&directory)?;
        let lock = acquire(&directory)?;
        remove_stale(&directory)?;
        let pinned = format!("/proc/self/fd/{}/broker.sock", directory.as_raw_fd());
        let listener = Listener::bind(Path::new(&pinned), backlog)?;
        let metadata = socket_metadata(&directory)?
            .ok_or_else(|| invalid("bound broker socket disappeared"))?;
        Ok(Self {
            listener,
            directory,
            _lock: lock,
            identity: (metadata.st_dev, metadata.st_ino),
        })
    }

    pub fn accept(&self, expected_uid: u32) -> io::Result<Option<(Socket, Caller)>> {
        self.listener.accept(expected_uid)
    }
}

impl AsRawFd for Owner {
    fn as_raw_fd(&self) -> RawFd {
        self.listener.as_raw_fd()
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        if let Ok(Some(metadata)) = socket_metadata(&self.directory) {
            if (metadata.st_dev, metadata.st_ino) == self.identity {
                let _ = unlink_socket(&self.directory);
            }
        }
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

fn validate_directory(directory: &File) -> io::Result<()> {
    let metadata = directory.metadata()?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o7777 != 0o700
    {
        return Err(invalid("broker directory must be owned private mode 0700"));
    }
    Ok(())
}

fn acquire(directory: &File) -> io::Result<File> {
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            LOCK.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0o600,
        )
    };
    if descriptor < 0 {
        return Err(io::Error::last_os_error());
    }
    let lock = unsafe { File::from_raw_fd(descriptor) };
    let metadata = lock.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.nlink() != 1
        || metadata.mode() & 0o7777 != 0o600
    {
        return Err(invalid(
            "broker lock must be owned exclusive mode 0600 file",
        ));
    }
    lock.try_lock().map_err(io::Error::from)?;
    Ok(lock)
}

fn socket_metadata(directory: &File) -> io::Result<Option<libc::stat>> {
    let mut metadata: libc::stat = unsafe { std::mem::zeroed() };
    let result = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            SOCKET.as_ptr(),
            &mut metadata,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        let error = io::Error::last_os_error();
        return if error.kind() == io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error)
        };
    }
    if metadata.st_mode & libc::S_IFMT != libc::S_IFSOCK
        || metadata.st_uid != unsafe { libc::geteuid() }
        || metadata.st_nlink != 1
    {
        return Err(invalid("broker endpoint is not an owned exclusive socket"));
    }
    Ok(Some(metadata))
}

fn unlink_socket(directory: &File) -> io::Result<()> {
    if unsafe { libc::unlinkat(directory.as_raw_fd(), SOCKET.as_ptr(), 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn remove_stale(directory: &File) -> io::Result<()> {
    if socket_metadata(directory)?.is_some() {
        unlink_socket(directory)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "owner_tests.rs"]
mod tests;
