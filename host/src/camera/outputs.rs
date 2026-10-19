//! Original output-device admission and shared camera frame policy.
use anyhow::{ensure, Context, Result};
use blent_config::camera::CameraProfile;
use std::{
    fs::File,
    os::unix::{
        fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
        io::AsRawFd,
    },
    path::Path,
    sync::Arc,
    time::Duration,
};
use tokio::sync::watch;

pub type Frames = watch::Sender<Option<Arc<Vec<u8>>>>;

pub fn open_device(path: &Path, label: &str) -> Result<File> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .with_context(|| {
            format!(
                "Open {}: install/load v4l2loopback and allow webcam access; see docs/cameras.md",
                path.display()
            )
        })?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.file_type().is_char_device(),
        "camera output must be a character device"
    );
    let name = format!(
        "/sys/dev/char/{}:{}/name",
        libc::major(metadata.rdev()),
        libc::minor(metadata.rdev())
    );
    ensure!(
        std::fs::read_to_string(name)?.trim() == label,
        "camera output must be labelled {label}"
    );
    let locked = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    ensure!(
        locked == 0,
        "camera output is already owned by another Blent camera process"
    );
    Ok(file)
}

pub fn blank(options: &CameraProfile) -> Result<Vec<u8>> {
    let bytes = options.frame_bytes()?;
    let pixels = bytes / 3 * 2;
    let mut frame = vec![128; bytes];
    frame[..pixels].fill(16);
    Ok(frame)
}

pub(super) const FRESH_FRAME_AGE: Duration = Duration::from_secs(2);

pub(super) fn current_frame<'a>(
    frame: Option<&'a [u8]>,
    black: &'a [u8],
    age: Duration,
) -> &'a [u8] {
    match frame {
        Some(bytes) if age < FRESH_FRAME_AGE => bytes,
        _ => black,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t539_inactive_and_stale_outputs_never_replay_camera_frames() {
        let black = [16, 128, 128];
        let live = vec![99, 99, 99];
        assert_eq!(current_frame(Some(&live), &black, Duration::ZERO), &live);
        assert_eq!(
            current_frame(Some(&live), &black, Duration::from_secs(2)),
            &black
        );
        assert_eq!(current_frame(None, &black, Duration::ZERO), &black);
        let root = tempfile::tempdir().unwrap();
        assert!(open_device(&root.path().join("missing"), "Blent Front").is_err());
        let path = root.path().join("regular");
        std::fs::write(&path, "").unwrap();
        assert!(open_device(&path, "Blent Front").is_err());
        assert!(open_device(Path::new("/dev/null"), "Blent Front").is_err());
    }
}
