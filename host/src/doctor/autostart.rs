//! Read-only Linux autostart observations for doctor. Nonzero systemctl exits
//! can carry valid states; absence and unavailable observations remain distinct.
use blent_config::commands::AsyncCommandExt;
use std::{io::Read, os::unix::fs::OpenOptionsExt, path::Path};

const MAX_DESKTOP_BYTES: u64 = 64 * 1024;

pub(super) async fn state() -> Option<String> {
    let output = tokio::process::Command::new("systemctl")
        .args(["--user", "is-enabled", "blent.service"])
        .output_bounded()
        .await
        .ok()?;
    state_text(&output.stdout)
}

pub(super) fn state_text(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?.trim();
    if text.is_empty()
        || text.len() > 64
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
    {
        return None;
    }
    Some(text.to_owned())
}

pub(super) fn desktop() -> Option<bool> {
    let path = blent_config::linux::autostart::desktop_path().ok()?;
    desktop_at(&path).ok()
}

pub(super) fn desktop_at(path: &Path) -> std::io::Result<bool> {
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_DESKTOP_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "desktop entry must be a regular file of at most 64 KiB",
        ));
    }
    let mut text = String::new();
    file.take(MAX_DESKTOP_BYTES + 1).read_to_string(&mut text)?;
    if text.len() as u64 > MAX_DESKTOP_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "desktop entry grew beyond 64 KiB",
        ));
    }
    Ok(blent_config::linux::autostart::desktop_enabled(&text))
}
