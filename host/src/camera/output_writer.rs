//! Linux output adapter. The owning worker must retain this writer until its
//! synchronous native call returns; aborting a future is not that fence.
use super::output_device::{ConfiguredOutput, Spec};
use std::{
    fs::File,
    io::{self, Write},
};

/// One original configured file description. Frames are borrowed only through
/// the synchronous write. No paths, background work or frame allocations occur.
/// Any native failure poisons the writer: a partial frame cannot be retried as
/// another complete V4L2 frame. Invalid input has no native effect.
pub struct Writer {
    output: File,
    spec: Spec,
    failed: bool,
}
impl Writer {
    pub fn new(configured: ConfiguredOutput) -> Self {
        let (output, spec) = configured.into_parts();
        Self {
            output,
            spec,
            failed: false,
        }
    }

    /// Attempt exactly one native frame write. A short/zero write or error is
    /// terminal, including EINTR; this adapter never retries an uncertain frame.
    pub fn write(&mut self, frame: &[u8]) -> io::Result<()> {
        self.write_with(frame, |mut file, bytes| file.write(bytes))
    }

    fn write_with(
        &mut self,
        frame: &[u8],
        native: impl FnOnce(&File, &[u8]) -> io::Result<usize>,
    ) -> io::Result<()> {
        if self.failed {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "camera writer retired after native failure",
            ));
        }
        if frame.len() != self.spec.frame_bytes() as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "camera output requires one complete frame",
            ));
        }
        self.failed = true;
        let count = native(&self.output, frame)?;
        if count != frame.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "camera output did not write one complete frame",
            ));
        }
        self.failed = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
