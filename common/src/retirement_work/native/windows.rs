use std::{os::windows::io::AsRawHandle, thread::JoinHandle};
use windows_sys::Win32::{Foundation::WAIT_OBJECT_0, System::Threading::WaitForSingleObject};

pub(crate) struct Join(Option<JoinHandle<()>>);
impl Join {
    pub(crate) fn new(handle: JoinHandle<()>) -> Self {
        Self(Some(handle))
    }
    pub(crate) fn poll_complete(&mut self) -> bool {
        let Some(handle) = &self.0 else {
            return true;
        };
        // SAFETY: the original thread handle remains owned until native exit.
        // A zero wait cannot block the shared reaper; failure retains ownership.
        if unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) } != WAIT_OBJECT_0 {
            return false;
        }
        let _ = self.0.take().unwrap().join();
        true
    }
}
