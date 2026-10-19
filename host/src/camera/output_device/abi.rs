//! Native Linux UAPI representation; independent header comparisons live in tests.
use super::Spec;
use anyhow::{ensure, Result};

pub(super) const OUTPUT: u32 = 2;
pub(super) const YUV420: u32 = u32::from_le_bytes(*b"YU12");
const PROGRESSIVE: u32 = 1;
const TIME_PER_FRAME: u32 = 0x1000;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct Pixels {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub field: u32,
    pub stride: u32,
    pub bytes: u32,
    pub colorspace: u32,
    pub private: u32,
    pub flags: u32,
    pub encoding: u32,
    pub quantization: u32,
    pub transfer: u32,
}

#[repr(C)]
pub(super) union Image {
    pub pixels: Pixels,
    raw: [u8; 200],
    // v4l2_window gives the native UAPI union its pointer alignment.
    alignment: *mut libc::c_void,
}

#[repr(C)]
pub(super) struct Format {
    pub kind: u32,
    #[cfg(target_pointer_width = "64")]
    padding: u32,
    pub image: Image,
}
impl Format {
    pub fn new(spec: Spec) -> Self {
        let mut image = Image { raw: [0; 200] };
        image.pixels = Pixels {
            width: spec.width,
            height: spec.height,
            format: YUV420,
            field: PROGRESSIVE,
            stride: spec.width,
            bytes: spec.bytes,
            ..Pixels::default()
        };
        Self {
            kind: OUTPUT,
            #[cfg(target_pointer_width = "64")]
            padding: 0,
            image,
        }
    }
    pub fn validate(&self, spec: Spec) -> Result<()> {
        ensure!(self.kind == OUTPUT, "camera output changed buffer type");
        // Every byte starts initialized; Pixels contains only all-bit-pattern u32s.
        let pixels = unsafe { self.image.pixels };
        ensure!(
            pixels.width == spec.width && pixels.height == spec.height,
            "camera output adjusted image dimensions"
        );
        ensure!(
            pixels.format == YUV420 && pixels.field == PROGRESSIVE,
            "camera output changed planar progressive YUV420"
        );
        ensure!(
            pixels.stride == spec.width && pixels.bytes == spec.bytes,
            "camera output requires incompatible frame padding"
        );
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct Fraction {
    pub numerator: u32,
    pub denominator: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct Rate {
    pub capability: u32,
    pub mode: u32,
    pub period: Fraction,
    pub extended_mode: u32,
    pub buffers: u32,
    pub reserved: [u32; 4],
}

#[repr(C)]
pub(super) union Timing {
    pub output: Rate,
    raw: [u8; 200],
}

#[repr(C)]
pub(super) struct Parameters {
    pub kind: u32,
    pub timing: Timing,
}
impl Parameters {
    pub fn new(fps: u32) -> Self {
        let mut timing = Timing { raw: [0; 200] };
        timing.output = Rate {
            period: Fraction {
                numerator: 1,
                denominator: fps,
            },
            ..Rate::default()
        };
        Self {
            kind: OUTPUT,
            timing,
        }
    }
    pub fn matches(&self, fps: u32) -> Result<bool> {
        ensure!(
            self.kind == OUTPUT,
            "camera output changed rate buffer type"
        );
        // The union is initialized throughout; Rate contains only u32 values.
        let period = unsafe { self.timing.output.period };
        ensure!(
            period.numerator != 0 && period.denominator != 0,
            "camera output returned an unknown frame interval"
        );
        Ok(u64::from(period.numerator) * u64::from(fps) == u64::from(period.denominator))
    }
    pub fn can_set(&self) -> bool {
        unsafe { self.timing.output.capability & TIME_PER_FRAME != 0 }
    }
}
