//! Native cross-process Wi-Fi attempt lease. Fixed stripes bound file count.
use anyhow::{ensure, Context, Result};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

pub(super) fn acquire(address: &str) -> Result<Box<dyn Send>> {
    let path = blent_config::platform::runtime::runtime_dir()?;
    #[cfg(windows)]
    let directory = blent_config::windows::private::Directory::create(&path)?;
    let file = at(&path, address)?;
    #[cfg(windows)]
    return Ok(Box::new((file, directory)));
    #[cfg(not(windows))]
    Ok(Box::new(file))
}
fn stripe(address: &str) -> u64 {
    let canonical = address
        .parse::<std::net::SocketAddr>()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| address.to_ascii_lowercase());
    canonical
        .bytes()
        .fold(14695981039346656037u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(1099511628211)
        })
        % 64
}
fn at(directory: &Path, address: &str) -> Result<File> {
    ensure!(crate::wifi::valid_address(address), "Invalid Wi-Fi address");
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(directory.join(format!("wifi-attempt-{}.lock", stripe(address))))?;
    ensure!(
        file.metadata()?.is_file(),
        "Wi-Fi lease is not a regular file"
    );
    file.try_lock()
        .context("Wi-Fi address stripe active in another process")?;
    Ok(file)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t834_native_lease_rejects_overlap_and_retains_stable_bounded_files() {
        let root = tempfile::tempdir().unwrap();
        let first = at(root.path(), "host:5").unwrap();
        assert!(at(root.path(), "HOST:5").is_err());
        assert_eq!(
            stripe("[2001:db8::1]:5"),
            stripe("[2001:0db8:0:0:0:0:0:1]:5")
        );
        drop(first);
        assert!(at(root.path(), "host:5").is_ok());
        assert!(at(root.path(), "host:0").is_err());
        for port in 1..=512 {
            assert!(stripe(&format!("host:{port}")) < 64);
        }
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn t834_native_lease_refuses_symlink() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("untouched");
        std::fs::write(&target, b"unchanged").unwrap();
        std::os::unix::fs::symlink(
            &target,
            root.path()
                .join(format!("wifi-attempt-{}.lock", stripe("host:5"))),
        )
        .unwrap();
        assert!(at(root.path(), "host:5").is_err());
        assert_eq!(std::fs::read(target).unwrap(), b"unchanged");
    }
}
