//! Fixed-layout YUV420 transforms. One session owns one bounded scratch frame;
//! callers exclusively borrow unpublished pixels. One reusable column table is
//! bounded by the validated width (1920 entries). No reader lease is retained.
use super::{adjustments::Image, CameraProfile};
use anyhow::{ensure, Result};
pub struct Transform {
    width: usize,
    height: usize,
    scratch: Vec<u8>,
    columns: Vec<usize>,
}
impl Transform {
    pub fn new(profile: &CameraProfile) -> Result<Self> {
        let bytes = profile.frame_bytes()?;
        Ok(Self {
            width: profile.width as usize,
            height: profile.height as usize,
            scratch: vec![0; bytes],
            columns: vec![0; profile.width as usize],
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
            &mut self.columns,
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
                &mut self.columns,
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
    let (w, h) = if rotation.is_multiple_of(180) {
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
fn copy_row(source: &[u8], output: &mut [u8], reverse: bool) {
    if reverse {
        for (to, from) in output.iter_mut().zip(source.iter().rev()) {
            *to = *from;
        }
    } else {
        output.copy_from_slice(source);
    }
}

fn straight(source: &[u8], output: &mut [u8], size: (usize, usize), image: Image) {
    let vertical = image.rotation == 180;
    let reverse = image.mirror != vertical;
    for (y, row) in output.chunks_exact_mut(size.0).enumerate() {
        let source_y = if vertical { size.1 - 1 - y } else { y };
        let start = source_y * size.0;
        copy_row(&source[start..start + size.0], row, reverse);
    }
}

fn column_offsets(columns: &mut [usize], size: (usize, usize), image: Image) {
    let count = columns.len();
    for (x, offset) in columns.iter_mut().enumerate() {
        let scaled = x * size.1 / count;
        let mirrored_y = if image.mirror {
            size.1 - 1 - scaled
        } else {
            scaled
        };
        let source_y = if image.rotation == 90 {
            size.1 - 1 - mirrored_y
        } else {
            mirrored_y
        };
        *offset = source_y * size.0;
    }
}

fn quarter(
    source: &[u8],
    output: &mut [u8],
    size: (usize, usize),
    rect: Rect,
    image: Image,
    black: u8,
    columns: &mut [usize],
) {
    output.fill(black);
    let offsets = &mut columns[..rect.width];
    column_offsets(offsets, size, image);
    for y in 0..rect.height {
        let scaled = y * size.0 / rect.height;
        let source_x = if image.rotation == 90 {
            scaled
        } else {
            size.0 - 1 - scaled
        };
        let start = (rect.top + y) * size.0 + rect.left;
        for (to, offset) in output[start..start + rect.width]
            .iter_mut()
            .zip(offsets.iter())
        {
            *to = source[source_x + offset];
        }
    }
}

fn plane(
    source: &[u8],
    output: &mut [u8],
    size: (usize, usize),
    rect: Rect,
    image: Image,
    black: u8,
    columns: &mut [usize],
) {
    if image.rotation.is_multiple_of(180) {
        straight(source, output, size, image);
    } else {
        quarter(source, output, size, rect, image, black, columns);
    }
}
#[cfg(test)]
mod oracle_tests;
#[cfg(test)]
mod tests;
