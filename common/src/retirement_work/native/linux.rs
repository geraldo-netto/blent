use std::{
    ffi::{c_int, c_void},
    os::unix::thread::{JoinHandleExt, RawPthread},
};

unsafe extern "C" {
    fn pthread_tryjoin_np(thread: RawPthread, result: *mut *mut c_void) -> c_int;
    fn pthread_detach(thread: RawPthread) -> c_int;
}

pub(crate) struct Join(Option<RawPthread>);
impl Join {
    pub(crate) fn new(handle: std::thread::JoinHandle<()>) -> Self {
        // Only Cleanup constructs this adapter. That worker returns unit and
        // stores all application/report panic payloads in its retained Shared
        // owner before returning. The Rust result packet contains no payload.
        Self(Some(handle.into_pthread_t()))
    }
    pub(crate) fn poll_complete(&mut self) -> bool {
        let Some(thread) = self.0 else {
            return true;
        };
        // SAFETY: this exact joinable pthread remains uniquely owned here.
        // Zero consumes its join right only after all native/TLS teardown.
        if unsafe { pthread_tryjoin_np(thread, std::ptr::null_mut()) } != 0 {
            return false;
        }
        self.0 = None;
        true
    }
}
impl Drop for Join {
    fn drop(&mut self) {
        if let Some(thread) = self.0.take() {
            // Matches JoinHandle's cancellation behavior for a caller dropping
            // an unsubmitted owner. Retained records never drop before join.
            // SAFETY: an unconsumed join right is still uniquely owned here.
            unsafe {
                pthread_detach(thread);
            }
        }
    }
}
