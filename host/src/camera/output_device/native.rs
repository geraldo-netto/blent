use super::abi::{Format, Parameters};
use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd},
};

pub(super) trait Controls {
    fn set_format(&self, output: BorrowedFd<'_>, format: &mut Format) -> io::Result<()>;
    fn get_rate(&self, output: BorrowedFd<'_>, parameters: &mut Parameters) -> io::Result<()>;
    fn set_rate(&self, output: BorrowedFd<'_>, parameters: &mut Parameters) -> io::Result<()>;
}
pub(super) struct Ioctl;
impl Controls for Ioctl {
    fn set_format(&self, output: BorrowedFd<'_>, format: &mut Format) -> io::Result<()> {
        // Borrowed descriptor and exclusive native payload remain live through return.
        checked(unsafe {
            libc::ioctl(
                output.as_raw_fd(),
                linux_raw_sys::ioctl::VIDIOC_S_FMT as _,
                std::ptr::from_mut(format),
            )
        })
    }
    fn get_rate(&self, output: BorrowedFd<'_>, parameters: &mut Parameters) -> io::Result<()> {
        checked(unsafe {
            libc::ioctl(
                output.as_raw_fd(),
                linux_raw_sys::ioctl::VIDIOC_G_PARM as _,
                std::ptr::from_mut(parameters),
            )
        })
    }
    fn set_rate(&self, output: BorrowedFd<'_>, parameters: &mut Parameters) -> io::Result<()> {
        checked(unsafe {
            libc::ioctl(
                output.as_raw_fd(),
                linux_raw_sys::ioctl::VIDIOC_S_PARM as _,
                std::ptr::from_mut(parameters),
            )
        })
    }
}
fn checked(result: libc::c_int) -> io::Result<()> {
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1143_native_ioctl_result_boundaries_keep_error_and_success_distinct() {
        for result in [i32::MIN, -1, 0, 1, i32::MAX] {
            assert_eq!(checked(result).is_ok(), result >= 0);
        }
    }
}
