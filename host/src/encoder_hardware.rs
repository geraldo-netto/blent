//! Linux stock-libavcodec hardware-frame boundary. CPU input remains borrowed
//! and immutable through upload; the codec retains independently owned surfaces.
use anyhow::{ensure, Context, Result};
#[cfg(not(test))]
use ffmpeg_next::ffi;
use ffmpeg_next::{format::Pixel, frame::Video};
#[cfg(test)]
#[path = "encoder_hardware_api_tests.rs"]
mod test_api;
use std::{ffi::CString, ptr};
#[cfg(test)]
use test_api as ffi;

struct Buffer(*mut ffi::AVBufferRef);

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { ffi::av_buffer_unref(&mut self.0) };
    }
}

fn checked(result: i32, operation: &'static str) -> Result<()> {
    if result < 0 {
        return Err(ffmpeg_next::Error::from(result)).context(operation);
    }
    Ok(())
}

fn device(path: &str) -> Result<Buffer> {
    let path = CString::new(path).context("VAAPI device contains NUL")?;
    let mut device = Buffer(ptr::null_mut());
    unsafe {
        checked(
            ffi::av_hwdevice_ctx_create(
                &mut device.0,
                ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI,
                path.as_ptr(),
                ptr::null_mut(),
                0,
            ),
            "open selected VAAPI device",
        )?;
    }
    Ok(device)
}

pub(super) struct Frames {
    pool: Buffer,
    width: u32,
    height: u32,
}

impl Frames {
    pub(super) fn new(path: &str, width: u32, height: u32) -> Result<Self> {
        blent_config::raw_frame::Layout::new(width, height, 2)?;
        let device = device(path)?;
        let pool = unsafe { Buffer(ffi::av_hwframe_ctx_alloc(device.0)) };
        ensure!(!pool.0.is_null(), "allocate VAAPI frame pool");
        unsafe {
            let context = &mut *((*pool.0).data.cast::<ffi::AVHWFramesContext>());
            context.format = Pixel::VAAPI.into();
            context.sw_format = Pixel::NV12.into();
            context.width = width as i32;
            context.height = height as i32;
            context.initial_pool_size = 4;
            checked(
                ffi::av_hwframe_ctx_init(pool.0),
                "initialize VAAPI frame pool",
            )?;
        }
        Ok(Self {
            pool,
            width,
            height,
        })
    }

    pub(super) fn attach(&self, encoder: &mut ffmpeg_next::encoder::video::Video) -> Result<()> {
        unsafe {
            let context = &mut *encoder.as_mut_ptr();
            ensure!(
                context.hw_frames_ctx.is_null(),
                "encoder already owns hardware frames"
            );
            let reference = ffi::av_buffer_ref(self.pool.0);
            ensure!(!reference.is_null(), "retain VAAPI frame pool");
            context.hw_frames_ctx = reference;
            context.pix_fmt = Pixel::VAAPI.into();
        }
        Ok(())
    }

    pub(super) fn upload(&self, source: &Video) -> Result<Video> {
        ensure!(source.format() == Pixel::NV12, "VAAPI input must be NV12");
        ensure!(
            source.width() == self.width && source.height() == self.height,
            "VAAPI input dimensions changed"
        );
        ensure!(
            plane_valid(source, 0, self.width, self.height),
            "invalid VAAPI luma plane"
        );
        ensure!(
            plane_valid(source, 1, self.width, self.height / 2),
            "invalid VAAPI chroma plane"
        );
        let mut frame = Video::empty();
        unsafe {
            ensure!(!frame.as_ptr().is_null(), "allocate VAAPI frame");
            checked(
                ffi::av_hwframe_get_buffer(self.pool.0, frame.as_mut_ptr(), 0),
                "acquire VAAPI surface",
            )?;
            checked(
                ffi::av_hwframe_transfer_data(frame.as_mut_ptr(), source.as_ptr(), 0),
                "upload NV12 frame",
            )?;
            checked(
                ffi::av_frame_copy_props(frame.as_mut_ptr(), source.as_ptr()),
                "copy VAAPI frame metadata",
            )?;
        }
        Ok(frame)
    }
}

fn plane_valid(frame: &Video, plane: usize, width: u32, rows: u32) -> bool {
    if plane > 1 || rows == 0 {
        return false;
    }
    unsafe {
        let raw = frame.as_ptr();
        let data = (*raw).data[plane];
        let stride = (*raw).linesize[plane];
        if data.is_null() || stride < 1 || (stride as u32) < width {
            return false;
        }
        let buffer = ffi::av_frame_get_plane_buffer(raw.cast_mut(), plane as i32);
        let Some(buffer) = buffer.as_ref() else {
            return false;
        };
        let extent = (u64::from(rows) - 1) * stride as u64 + u64::from(width);
        let offset = (data as usize).checked_sub(buffer.data as usize);
        offset
            .is_some_and(|offset| offset <= buffer.size && extent <= (buffer.size - offset) as u64)
    }
}

#[cfg(test)]
#[path = "encoder_hardware_tests.rs"]
mod tests;
