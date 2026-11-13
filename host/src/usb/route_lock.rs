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
    let canonical = crate::wifi::endpoint_key(address);
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
/// Fixed stripes bound persistent lock files. Collisions reject admission;
/// locks are never unlinked, so waiters cannot acquire a replacement inode.
pub(super) fn display(identity: &str, ports: (u16, u16)) -> Result<Box<dyn Send>> {
    let directory = blent_config::platform::runtime::runtime_dir()?;
    #[cfg(windows)]
    let private = blent_config::windows::private::Directory::create(&directory)?;
    let mut keys = vec![
        format!("device:{identity}"),
        format!("port:{}", ports.0),
        format!("port:{}", ports.1),
    ];
    let mut stripes: Vec<_> = keys
        .drain(..)
        .map(|key| {
            key.bytes().fold(0u64, |hash, byte| {
                hash.wrapping_mul(31).wrapping_add(u64::from(byte))
            }) % 256
        })
        .collect();
    stripes.sort_unstable();
    stripes.dedup();
    let files: Result<Vec<_>> = stripes
        .into_iter()
        .map(|stripe| display_file(&directory, stripe))
        .collect();
    let files = files?;
    #[cfg(windows)]
    return Ok(Box::new((files, private)));
    #[cfg(not(windows))]
    Ok(Box::new(files))
}
fn display_file(directory: &Path, stripe: u64) -> Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(directory.join(format!("display-route-{stripe}.lock")))?;
    ensure!(
        file.metadata()?.is_file(),
        "Display lease is not a regular file"
    );
    file.try_lock()
        .context("Display route namespace remains owned")?;
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
    #[test]
    fn t1344_native_display_leases_retain_devices_and_local_ports() {
        let commands = super::super::NativeCommands("unused".into());
        use super::super::Commands;
        let first = commands
            .display_route_lease("T1344-owner", (61234, 61235))
            .unwrap();
        assert!(commands
            .display_route_lease("T1344-owner", (61236, 61237))
            .is_err());
        assert!(commands
            .display_route_lease("T1344-other", (61234, 61235))
            .is_err());
        drop(first);
        assert!(Some(commands)
            .display_route_lease("T1344-owner", (61234, 61235))
            .is_ok());
        assert!(None::<super::super::NativeCommands>
            .display_route_lease("T1344-owner", (61234, 61235))
            .is_err());
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn t1344_display_lease_rejects_symlink_without_unlinking() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target");
        std::fs::write(&target, b"preserved").unwrap();
        std::os::unix::fs::symlink(&target, root.path().join("display-route-0.lock")).unwrap();
        assert!(display_file(root.path(), 0).is_err());
        assert_eq!(std::fs::read(target).unwrap(), b"preserved");
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
