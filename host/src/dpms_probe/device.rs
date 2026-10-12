use super::library::Library;
use super::*;
use anyhow::Context;
use std::{
    ffi::{c_char, c_int, c_void},
    ptr::NonNull,
};
type Open = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type Close = unsafe extern "C" fn(*mut c_void) -> c_int;
type Query = unsafe extern "C" fn(*mut c_void, *mut c_int, *mut c_int) -> c_int;
type Capable = unsafe extern "C" fn(*mut c_void) -> c_int;
type Info = unsafe extern "C" fn(*mut c_void, *mut u16, *mut u8) -> c_int;
struct Api {
    open: Open,
    close: Close,
    query: Query,
    capable: Capable,
    info: Info,
}
pub(super) struct Device {
    display: NonNull<c_void>,
    api: Api,
    // Code remains mapped until the display and every call have retired.
    _libraries: Option<(Library, Library)>,
}
impl Device {
    pub fn open() -> Result<Self> {
        let x11 = Library::open(c"libX11.so.6")?;
        let extension = Library::open(c"libXext.so.6")?;
        let api = unsafe {
            Api {
                open: x11.symbol(c"XOpenDisplay")?,
                close: x11.symbol(c"XCloseDisplay")?,
                query: extension.symbol(c"DPMSQueryExtension")?,
                capable: extension.symbol(c"DPMSCapable")?,
                info: extension.symbol(c"DPMSInfo")?,
            }
        };
        Self::with_api(api, Some((x11, extension)))
    }
    fn with_api(api: Api, libraries: Option<(Library, Library)>) -> Result<Self> {
        let display = unsafe { (api.open)(std::ptr::null()) };
        Ok(Self {
            display: NonNull::new(display).context("X11 display unavailable")?,
            api,
            _libraries: libraries,
        })
    }
    pub fn sample(&self) -> Result<DisplayObservation> {
        let mut event = 0;
        let mut error = 0;
        let extension = unsafe { (self.api.query)(self.display.as_ptr(), &mut event, &mut error) };
        if !boolean(extension)? {
            return Ok(DisplayObservation {
                supported: false,
                on: None,
            });
        }
        let capable = unsafe { (self.api.capable)(self.display.as_ptr()) };
        if !boolean(capable)? {
            return Ok(DisplayObservation {
                supported: false,
                on: None,
            });
        }
        let mut level = 0;
        let mut enabled = 0;
        let status = unsafe { (self.api.info)(self.display.as_ptr(), &mut level, &mut enabled) };
        ensure!(status == 1, "DPMS state query failed");
        observation(boolean(enabled.into())?, level)
    }
}
fn boolean(value: c_int) -> Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => anyhow::bail!("invalid native DPMS boolean"),
    }
}
fn observation(enabled: bool, level: u16) -> Result<DisplayObservation> {
    // The protocol makes level undefined when DPMS is disabled; the display is on.
    let on = if !enabled {
        true
    } else {
        ensure!(level <= 3, "invalid native DPMS power level");
        level == 0
    };
    Ok(DisplayObservation {
        supported: true,
        on: Some(on),
    })
}
impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            (self.api.close)(self.display.as_ptr());
        }
    }
}
#[cfg(test)]
pub(super) mod tests;
