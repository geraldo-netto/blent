//! Native pipe readiness. Only the collection worker reads each owned endpoint.
use std::io::{self, PipeReader, Read};
pub(super) struct Reader(PipeReader);
impl Reader {
    pub fn new(pipe: PipeReader) -> io::Result<Self> {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            let flags = unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL) };
            if flags < 0
                || unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) }
                    < 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(Self(pipe))
    }
}
impl Read for Reader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::{Foundation::ERROR_BROKEN_PIPE, System::Pipes::PeekNamedPipe};
            let mut available = 0;
            let ok = unsafe {
                PeekNamedPipe(
                    self.0.as_raw_handle(),
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut available,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                let error = io::Error::last_os_error();
                return if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
                    Ok(0)
                } else {
                    Err(error)
                };
            }
            if available == 0 {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            let length = bytes.len().min(available as usize);
            return self.0.read(&mut bytes[..length]);
        }
        #[cfg(not(windows))]
        self.0.read(bytes)
    }
}
