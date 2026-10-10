use super::{invalid, owned, MAX_FDS};
use std::io;
use std::os::fd::{OwnedFd, RawFd};

pub(super) fn sending(header: &mut libc::msghdr, buffer: &mut [usize; 16], fds: &[RawFd]) {
    if fds.is_empty() {
        return;
    }
    let bytes = std::mem::size_of_val(fds);
    header.msg_control = buffer.as_mut_ptr().cast();
    header.msg_controllen = unsafe { libc::CMSG_SPACE(bytes as u32) } as usize;
    unsafe {
        let message = libc::CMSG_FIRSTHDR(header);
        (*message).cmsg_level = libc::SOL_SOCKET;
        (*message).cmsg_type = libc::SCM_RIGHTS;
        (*message).cmsg_len = libc::CMSG_LEN(bytes as u32) as usize;
        for (index, fd) in fds.iter().enumerate() {
            std::ptr::write_unaligned(libc::CMSG_DATA(message).cast::<i32>().add(index), *fd);
        }
    }
}
/// Called only for the kernel-populated control buffer from recvmsg. Kernel
/// normalizes cmsg headers and closes excess truncated rights. Unknown records
/// are rejected AFTER collecting every delivered right, including later cmsgs.
pub(super) fn received(header: &libc::msghdr) -> io::Result<Vec<OwnedFd>> {
    let mut fds = Vec::new();
    let mut invalid_control = false;
    unsafe {
        let mut message = libc::CMSG_FIRSTHDR(header);
        while !message.is_null() {
            if (*message).cmsg_level == libc::SOL_SOCKET && (*message).cmsg_type == libc::SCM_RIGHTS
            {
                collect(message, &mut fds)?;
            } else {
                invalid_control = true;
            }
            message = libc::CMSG_NXTHDR(header, message);
        }
    }
    if invalid_control || fds.len() > MAX_FDS {
        return Err(invalid("unexpected local descriptor control"));
    }
    Ok(fds)
}
unsafe fn collect(message: *const libc::cmsghdr, fds: &mut Vec<OwnedFd>) -> io::Result<()> {
    let size = (*message)
        .cmsg_len
        .checked_sub(libc::CMSG_LEN(0) as usize)
        .ok_or_else(|| invalid("invalid descriptor control size"))?;
    if size % std::mem::size_of::<i32>() != 0 {
        return Err(invalid("unaligned descriptor control size"));
    }
    for offset in (0..size).step_by(std::mem::size_of::<i32>()) {
        fds.push(owned(std::ptr::read_unaligned(
            libc::CMSG_DATA(message).add(offset).cast::<i32>(),
        ))?);
    }
    Ok(())
}
