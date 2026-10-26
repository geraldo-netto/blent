//! Platform adapters for an exact, nonblocking native thread-exit fence.
//! Each adapter owns one original thread identity until a successful native join.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub(super) use linux::Join;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(super) use windows::Join;

// Unsupported application platforms cannot turn a Rust completion flag into
// native exit evidence. Retain the original owner instead of releasing capacity.
#[cfg(not(any(target_os = "linux", windows)))]
pub(super) struct Join {
    _handle: std::thread::JoinHandle<()>,
}
#[cfg(not(any(target_os = "linux", windows)))]
impl Join {
    pub(super) fn new(handle: std::thread::JoinHandle<()>) -> Self {
        Self { _handle: handle }
    }
    pub(super) fn poll_complete(&mut self) -> bool {
        false
    }
}
