//! Fixed-layout YUV420 transforms. One session owns one bounded scratch frame;
//! callers exclusively borrow unpublished pixels. No reader lease is retained.
use super::{adjustments::Image, CameraProfile};
use anyhow::{ensure, Result};
pub struct Transform {
    width: usize,
    height: usize,
    scratch: Vec<u8>,
}
impl Transform {
    pub fn new(profile: &CameraProfile) -> Result<Self> {
        Ok(Self {
            width: profile.width as usize,
            height: profile.height as usize,
            scratch: vec![0; profile.frame_bytes()?],
        })
    }
    pub fn apply(&mut self, image: Image, frame: &mut [u8]) -> Result<()> {
        ensure!(
            frame.len() == self.scratch.len(),
            "Camera transform length mismatch"
        );
        ensure!(
            matches!(image.rotation, 0 | 90 | 180 | 270),
            "Invalid camera rotation"
        );
        if image.rotation == 0 && !image.mirror {
            return Ok(());
        }
        self.scratch.copy_from_slice(frame);
        let pixels = self.width * self.height;
        let rect = placement(self.width, self.height, image.rotation);
        plane(
            &self.scratch[..pixels],
            &mut frame[..pixels],
            (self.width, self.height),
            rect,
            image,
            16,
        );
        for channel in 0..2 {
            let start = pixels + channel * pixels / 4;
            let end = start + pixels / 4;
            plane(
                &self.scratch[start..end],
                &mut frame[start..end],
                (self.width / 2, self.height / 2),
                rect.chroma(),
                image,
                128,
            );
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct Rect {
    left: usize,
    top: usize,
    width: usize,
    height: usize,
}
impl Rect {
    fn chroma(self) -> Self {
        Self {
            left: self.left / 2,
            top: self.top / 2,
            width: self.width / 2,
            height: self.height / 2,
        }
    }
}
fn placement(width: usize, height: usize, rotation: u16) -> Rect {
    let (w, h) = if rotation % 180 == 0 {
        (width, height)
    } else if width > height {
        ((height * height / width).max(2) & !1, height)
    } else {
        (width, (width * width / height).max(2) & !1)
    };
    Rect {
        left: ((width - w) / 2) & !1,
        top: ((height - h) / 2) & !1,
        width: w,
        height: h,
    }
}
fn point(x: usize, y: usize, width: usize, height: usize, image: Image) -> usize {
    let turned_width = if image.rotation % 180 == 0 {
        width
    } else {
        height
    };
    let x = if image.mirror {
        turned_width - 1 - x
    } else {
        x
    };
    let (x, y) = match image.rotation {
        90 => (y, height - 1 - x),
        180 => (width - 1 - x, height - 1 - y),
        270 => (width - 1 - y, x),
        _ => (x, y),
    };
    y * width + x
}
fn plane(
    source: &[u8],
    output: &mut [u8],
    size: (usize, usize),
    rect: Rect,
    image: Image,
    black: u8,
) {
    output.fill(black);
    let (width, height) = size;
    let (turned_width, turned_height) = if image.rotation % 180 == 0 {
        (width, height)
    } else {
        (height, width)
    };
    for y in 0..rect.height {
        for x in 0..rect.width {
            let index = point(
                x * turned_width / rect.width,
                y * turned_height / rect.height,
                width,
                height,
                image,
            );
            output[(rect.top + y) * width + rect.left + x] = source[index];
        }
    }
}
#[cfg(test)]
mod tests;
