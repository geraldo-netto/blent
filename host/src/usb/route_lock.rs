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
/// Device and port namespaces are independent. Retain stable inodes: unlinking
/// a lock could let a new caller bypass an existing holder on the old inode.
pub(super) fn display(identity: &str, ports: (u16, u16)) -> Result<Box<dyn Send>> {
    use sha2::Digest;
    let directory = blent_config::platform::runtime::runtime_dir()?;
    #[cfg(windows)]
    let private = blent_config::windows::private::Directory::create(&directory)?;
    let mut keys = vec![
        format!(
            "display-device-{}",
            crate::hash_text::encode(sha2::Sha256::digest(identity.as_bytes()).into())
        ),
        format!("display-port-{}", ports.0),
        format!("display-port-{}", ports.1),
    ];
    keys.sort_unstable();
    keys.dedup();
    let files: Result<Vec<_>> = keys
        .iter()
        .map(|key| display_file(&directory, key))
        .collect();
    let files = files?;
    #[cfg(windows)]
    return Ok(Box::new((files, private)));
    #[cfg(not(windows))]
    Ok(Box::new(files))
}
fn display_file(directory: &Path, key: &str) -> Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(directory.join(format!("{key}.lock")))?;
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
    #[cfg(target_os = "linux")]
    #[test]
    fn t1541_hash_collision_does_not_reserve_an_unrelated_tablet() {
        if crate::test_logging::isolated(
            "usb::route_lock::tests::t1541_hash_collision_does_not_reserve_an_unrelated_tablet",
        ) {
            return;
        }
        let _root = private_runtime();
        let first = display("T1541-Aa", (61000, 61001)).unwrap();
        let second = display("T1541-BB", (61002, 61003));
        assert!(
            second.is_ok(),
            "T1541 distinct tablets collide: {:?}",
            second.err()
        );
        assert!(display("T1541-Aa", (61004, 61005)).is_err());
        assert!(display("T1541-other", (61001, 61006)).is_err());
        drop(first);
        assert!(display("T1541-Aa", (61000, 61001)).is_ok());
    }
    #[cfg(target_os = "linux")]
    fn private_runtime() -> tempfile::TempDir {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        // Only called before starting threads in an isolated test child.
        unsafe {
            std::env::set_var("XDG_RUNTIME_DIR", root.path());
        }
        root
    }
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
        #[cfg(target_os = "linux")]
        if crate::test_logging::isolated(
            "usb::route_lock::tests::t1344_native_display_leases_retain_devices_and_local_ports",
        ) {
            return;
        }
        #[cfg(target_os = "linux")]
        let _root = private_runtime();
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
        std::os::unix::fs::symlink(&target, root.path().join("display-port-0.lock")).unwrap();
        assert!(display_file(root.path(), "display-port-0").is_err());
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
