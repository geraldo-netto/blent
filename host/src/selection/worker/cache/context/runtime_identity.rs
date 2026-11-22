//! Linux cache identity only: no execution, authority or retained native view.
//! Declared runtime files are rooted at the selected program's parent directory's
//! parent (the package's `usr`). One file is borrowed at a time; reads are bounded
//! to 512 MiB per component, 32 components, 16 KiB metadata and a 64 KiB buffer.
//! Opaque scripts without a declaration cannot supply a reliable tuning key.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

const MAX_FILE: u64 = 512 * 1024 * 1024;
const MAX_MANIFEST: u64 = 16384;
mod memo;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u8,
    files: Vec<String>,
}

fn open(path: &Path, limit: u64) -> Option<std::fs::File> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let metadata = file.metadata().ok()?;
    (metadata.is_file() && metadata.len() <= limit).then_some(file)
}

pub(super) fn file_hash(path: &Path) -> Option<[u8; 32]> {
    let path = path.canonicalize().ok()?;
    let mut file = open(&path, MAX_FILE)?;
    let stamp = memo::Stamp::from(&file.metadata().ok()?);
    let cached = memo::cache()
        .lock()
        .ok()?
        .lookup(&path, stamp, memo::capacity()?);
    if let Some(digest) = cached {
        return Some(digest);
    }
    let mut input = (&mut file).take(MAX_FILE + 1);
    let mut digest = Sha256::new();
    let mut chunk = [0; 65536];
    let mut total = 0;
    loop {
        let count = input.read(&mut chunk).ok()?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > MAX_FILE {
            return None;
        }
        digest.update(&chunk[..count]);
    }
    if memo::Stamp::from(&file.metadata().ok()?) != stamp {
        return None;
    }
    let digest = digest.finalize().into();
    memo::cache()
        .lock()
        .ok()?
        .remember(path, stamp, digest, memo::capacity()?);
    Some(digest)
}

fn manifest(path: &Path) -> Option<Manifest> {
    let mut bytes = Vec::new();
    open(path, MAX_MANIFEST)?
        .take(MAX_MANIFEST + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_MANIFEST {
        return None;
    }
    let manifest: Manifest = serde_json::from_slice(&bytes).ok()?;
    if manifest.schema != 1 || !(1..=32).contains(&manifest.files.len()) {
        return None;
    }
    if manifest.files.first()?.as_str() != "libexec/ffmpeg" {
        return None;
    }
    let unique: std::collections::BTreeSet<_> = manifest.files.iter().collect();
    (unique.len() == manifest.files.len()).then_some(manifest)
}

fn component(root: &Path, name: &str) -> Option<PathBuf> {
    if name.len() > 256 || name.chars().any(char::is_control) {
        return None;
    }
    if name.split('/').any(|part| matches!(part, "" | "." | "..")) {
        return None;
    }
    let path = root.join(name).canonicalize().ok()?;
    path.starts_with(root).then_some(path)
}

fn native_elf(program: &Path) -> bool {
    let Some(mut file) = open(program, MAX_FILE) else {
        return false;
    };
    let mut magic = [0; 4];
    file.read_exact(&mut magic).is_ok() && magic == *b"\x7fELF"
}

pub(super) fn fingerprint(program: &Path) -> Option<[u8; 32]> {
    let program = program.canonicalize().ok()?;
    let mut name = program.file_name()?.to_owned();
    name.push(".blent-runtime.json");
    let metadata = program.with_file_name(name);
    let mut digest = Sha256::new();
    digest.update(b"blent-ffmpeg-runtime-v1");
    digest.update(file_hash(&program)?);
    match std::fs::symlink_metadata(&metadata) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if !native_elf(&program) {
                return None;
            }
        }
        Ok(_) => hash_declared(&mut digest, &program, &metadata)?,
        Err(_) => return None,
    }
    Some(digest.finalize().into())
}

fn hash_declared(digest: &mut Sha256, program: &Path, metadata: &Path) -> Option<()> {
    let root = program.parent()?.parent()?;
    for name in manifest(metadata)?.files {
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        digest.update(file_hash(&component(root, &name)?)?);
    }
    Some(())
}

#[cfg(test)]
mod tests;
