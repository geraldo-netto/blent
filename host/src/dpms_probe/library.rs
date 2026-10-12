//! Runtime linking keeps unavailable native libraries visibly unsupported.
use anyhow::{ensure, Context, Result};
use std::{
    ffi::{c_void, CStr},
    ptr::NonNull,
};
pub(super) struct Library(NonNull<c_void>);
impl Library {
    pub fn open(name: &CStr) -> Result<Self> {
        let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        Ok(Self(
            NonNull::new(handle).context("native DPMS library unavailable")?,
        ))
    }
    /// # Safety
    /// T must be the exact function-pointer ABI of this library's named symbol.
    pub unsafe fn symbol<T: Copy>(&self, name: &CStr) -> Result<T> {
        ensure!(
            std::mem::size_of::<T>() == std::mem::size_of::<*mut c_void>(),
            "invalid native function ABI size"
        );
        let pointer = unsafe { libc::dlsym(self.0.as_ptr(), name.as_ptr()) };
        ensure!(!pointer.is_null(), "native DPMS symbol unavailable");
        Ok(unsafe { std::mem::transmute_copy(&pointer) })
    }
}
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            libc::dlclose(self.0.as_ptr());
        }
    }
}
