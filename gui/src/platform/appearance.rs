//! Linux boundary: bounded child receipts own GTK/fontconfig probes. Snapshot
//! strings <=256 bytes; output <=8 KiB; regular font input <=8 MiB. No listener.
use crate::appearance_contract::Appearance;
use blent_config::commands::SyncCommandExt;
use std::{io::Read, process::Command};

pub(crate) fn load() -> Result<(Appearance, Vec<u8>), String> {
    let output = Command::new("/usr/bin/python3")
        .args(["-I", "-c", include_str!("appearance/gtk.py")])
        .output_bounded()
        .map_err(|error| error.to_string())?;
    let appearance = parse(&output.stdout, output.status.success())?;
    let font = font(&appearance.font_family)?;
    Ok((appearance, font))
}

fn parse(bytes: &[u8], success: bool) -> Result<Appearance, String> {
    if !success || bytes.len() > 8192 {
        return Err("GTK light-theme probe unavailable".into());
    }
    let value: toml::Value = std::str::from_utf8(bytes)
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: toml::de::Error| e.to_string())?;
    let appearance = Appearance {
        source: text(&value, "source")?,
        background: color(&value, "background")?,
        surface: color(&value, "surface")?,
        foreground: color(&value, "foreground")?,
        accent: color(&value, "accent")?,
        selected_text: color(&value, "selected_text")?,
        font_family: text(&value, "font_family")?,
        font_points: number(&value, "font_points", 6.0..=48.0)?,
        text_scale: number(&value, "text_scale", 0.5..=4.0)?,
        animations: value
            .get("animations")
            .and_then(toml::Value::as_bool)
            .ok_or("Missing animation preference")?,
    };
    if appearance
        .background
        .iter()
        .map(|v| u16::from(*v))
        .sum::<u16>()
        < 384
    {
        return Err("Desktop theme did not supply a light palette".into());
    }
    Ok(appearance)
}

fn text(value: &toml::Value, key: &str) -> Result<String, String> {
    let text = value
        .get(key)
        .and_then(toml::Value::as_str)
        .ok_or("Missing theme text")?;
    if text.is_empty() || text.len() > 256 || text.chars().any(char::is_control) {
        return Err("Invalid theme text".into());
    }
    Ok(text.into())
}

fn number(
    value: &toml::Value,
    key: &str,
    range: std::ops::RangeInclusive<f32>,
) -> Result<f32, String> {
    let number = value
        .get(key)
        .and_then(toml::Value::as_float)
        .ok_or("Missing theme number")? as f32;
    if !range.contains(&number) {
        return Err("Theme number out of range".into());
    }
    Ok(number)
}

fn color(value: &toml::Value, key: &str) -> Result<[u8; 3], String> {
    let channels = value
        .get(key)
        .and_then(toml::Value::as_array)
        .ok_or("Missing theme color")?;
    if channels.len() != 3 {
        return Err("Invalid theme color length".into());
    }
    let mut rgb = [0; 3];
    for (out, value) in rgb.iter_mut().zip(channels) {
        *out = u8::try_from(value.as_integer().ok_or("Invalid theme color")?)
            .map_err(|e| e.to_string())?;
    }
    Ok(rgb)
}

fn font(family: &str) -> Result<Vec<u8>, String> {
    let output = Command::new("fc-match")
        .args(["--format=%{file}", "--", family])
        .output_bounded()
        .map_err(|error| error.to_string())?;
    if !output.status.success() || output.stdout.len() > 4096 {
        return Err("Desktop font unavailable".into());
    }
    let path = std::str::from_utf8(&output.stdout).map_err(|e| e.to_string())?;
    read_font(std::path::Path::new(path))
}

fn read_font(path: &std::path::Path) -> Result<Vec<u8>, String> {
    const LIMIT: u64 = 8 * 1024 * 1024;
    use std::os::unix::fs::OpenOptionsExt;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > LIMIT {
        return Err("Desktop font exceeds input limit".into());
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() as u64 > LIMIT {
        return Err("Invalid desktop font size".into());
    }
    ab_glyph::FontRef::try_from_slice(&bytes).map_err(|_| "Invalid desktop font data")?;
    Ok(bytes)
}

#[cfg(test)]
mod tests;
