//! Test-only foreign-API seam. Real AVBuffer/AVFrame ownership stays active;
//! device operations use CPU surfaces for portable lifecycle/failure coverage.
//! Explicit native acceptance disables this seam and exercises the real driver.
use ffmpeg_next::ffi as real;
pub use ffmpeg_next::ffi::*;
use std::{cell::Cell, ffi::c_char, ptr};

#[derive(Clone, Copy, Default)]
struct State {
    enabled: bool,
    failure: i32,
}
thread_local! { static STATE: Cell<State> = Cell::new(State::default()); }
pub struct Guard(State);
impl Drop for Guard {
    fn drop(&mut self) {
        STATE.set(self.0);
    }
}
pub fn activate() -> Guard {
    Guard(STATE.replace(State {
        enabled: true,
        failure: 0,
    }))
}
pub fn fail(failure: i32) {
    let mut state = STATE.get();
    state.failure = failure;
    STATE.set(state);
}
fn mode() -> Option<i32> {
    let state = STATE.get();
    state.enabled.then_some(state.failure)
}

pub unsafe fn av_hwdevice_ctx_create(
    output: *mut *mut AVBufferRef,
    kind: AVHWDeviceType,
    path: *const c_char,
    options: *mut AVDictionary,
    flags: i32,
) -> i32 {
    match mode() {
        None => real::av_hwdevice_ctx_create(output, kind, path, options, flags),
        Some(1) => AVERROR(libc::ENOMEM),
        Some(_) => {
            *output = real::av_buffer_allocz(1);
            assert!(!(*output).is_null());
            0
        }
    }
}

pub unsafe fn av_hwframe_ctx_alloc(device: *mut AVBufferRef) -> *mut AVBufferRef {
    match mode() {
        None => real::av_hwframe_ctx_alloc(device),
        Some(2) => ptr::null_mut(),
        Some(_) => real::av_buffer_allocz(std::mem::size_of::<AVHWFramesContext>()),
    }
}

pub unsafe fn av_hwframe_ctx_init(pool: *mut AVBufferRef) -> i32 {
    match mode() {
        None => real::av_hwframe_ctx_init(pool),
        Some(3) => AVERROR(libc::EINVAL),
        Some(_) => 0,
    }
}

pub unsafe fn av_buffer_ref(buffer: *const AVBufferRef) -> *mut AVBufferRef {
    if mode() == Some(4) {
        ptr::null_mut()
    } else {
        real::av_buffer_ref(buffer)
    }
}

pub unsafe fn av_hwframe_get_buffer(
    pool: *mut AVBufferRef,
    frame: *mut AVFrame,
    flags: i32,
) -> i32 {
    match mode() {
        None => real::av_hwframe_get_buffer(pool, frame, flags),
        Some(5) => AVERROR(libc::ENOMEM),
        Some(_) => {
            let context = &*((*pool).data.cast::<AVHWFramesContext>());
            (*frame).format = AVPixelFormat::AV_PIX_FMT_NV12 as i32;
            (*frame).width = context.width;
            (*frame).height = context.height;
            // Keep real reference ownership without asking FFmpeg to dispatch
            // hardware operations through this test-only CPU context.
            (*frame).opaque_ref = real::av_buffer_ref(pool);
            real::av_frame_get_buffer(frame, 32)
        }
    }
}

pub unsafe fn av_hwframe_transfer_data(
    destination: *mut AVFrame,
    source: *const AVFrame,
    flags: i32,
) -> i32 {
    match mode() {
        None => real::av_hwframe_transfer_data(destination, source, flags),
        Some(6) => AVERROR(libc::EIO),
        Some(_) => real::av_frame_copy(destination, source),
    }
}

pub unsafe fn av_frame_copy_props(destination: *mut AVFrame, source: *const AVFrame) -> i32 {
    if mode() == Some(7) {
        AVERROR(libc::ENOMEM)
    } else {
        real::av_frame_copy_props(destination, source)
    }
}
