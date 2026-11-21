use super::*;
use std::os::unix::fs::symlink;

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let dri = root.path().join("dri");
    let sys = root.path().join("sys");
    std::fs::create_dir_all(dri.join("by-path")).unwrap();
    std::fs::create_dir_all(sys.join("renderD128/device")).unwrap();
    std::fs::write(dri.join("renderD128"), "").unwrap();
    symlink("../renderD128", dri.join("by-path/pci-first-render")).unwrap();
    std::fs::write(
        sys.join("renderD128/device/uevent"),
        "DRIVER=amdgpu\nPCI_ID=1002:73FF\n",
    )
    .unwrap();
    (root, dri, sys)
}

#[test]
fn t727_discovery_checks_stable_identity_permissions_and_native_bounds() {
    let (root, dri, sys) = fixture();
    let devices = discover_in(&dri, &sys, |_| true);
    assert_eq!(devices.len(), 1);
    assert!(devices[0].adapter.label.starts_with("AMD "));
    assert_eq!(devices[0].adapter.id, "vaapi:pci-first-render");
    assert_eq!(devices[0].node, dri.join("renderD128"));
    assert!(catalog(&devices).supported);
    assert!(!discover_in(&dri, &sys, |_| false)[0].adapter.accessible);
    assert!(discover_in(&root.path().join("missing"), &sys, |_| true).is_empty());
    for name in ["card", "pci-card", "missing-render"] {
        std::fs::write(dri.join("by-path").join(name), "").unwrap();
    }
    symlink(root.path(), dri.join("by-path/outside-render")).unwrap();
    std::fs::write(dri.join("invalid"), "").unwrap();
    symlink("../invalid", dri.join("by-path/invalid-render")).unwrap();
    let long = format!("{}-render", "a".repeat(200));
    symlink("../renderD128", dri.join("by-path").join(long)).unwrap();
    assert_eq!(discover_in(&dri, &sys, |_| true).len(), 1);
    for (id, expected) in [
        ("8086:1234", "Intel"),
        ("10DE:0011", "NVIDIA"),
        ("ABCD:0000", "GPU"),
    ] {
        assert!(device_label("id", &format!("PCI_ID={id}"), "").starts_with(expected));
    }
    assert!(device_label("id", "", "").contains("unknown driver"));
    std::fs::remove_file(sys.join("renderD128/device/uevent")).unwrap();
    assert_eq!(discover_in(&dri, &sys, |_| true).len(), 1);
    let native = discover(); // Actual Linux inventory and read/write permission check.
    assert!(native
        .iter()
        .all(|device| device.node.starts_with("/dev/dri")));
}

#[test]
fn t727_effective_reports_require_owned_process_and_open_device_evidence() {
    let (root, dri, sys) = fixture();
    let devices = discover_in(&dri, &sys, |_| true);
    let proc = root.path().join("proc");
    let process_dir = proc.join("123");
    std::fs::create_dir_all(process_dir.join("fd")).unwrap();
    let mut process = super::super::processes::Process {
        pid: 123,
        uid: 0,
        start_ticks: 1,
        executable: "ffmpeg".into(),
        arguments: vec![
            "ffmpeg".into(),
            "-i".into(),
            "/owned/capture.fifo".into(),
            "-c:v".into(),
            "h264_vaapi".into(),
        ],
        cwd: "/".into(),
    };
    let fifos = vec!["/owned/capture.fifo".into()];
    assert!(observe(&proc, &process, 55, &fifos, &devices).is_none());
    for stat in ["invalid", "123 (ffmpeg) S unknown", "123 (ffmpeg) S 99"] {
        std::fs::write(process_dir.join("stat"), stat).unwrap();
        assert!(observe(&proc, &process, 55, &fifos, &devices).is_none());
    }
    std::fs::write(process_dir.join("stat"), "123 (ffmpeg) S 55").unwrap();
    assert!(observe(&proc, &process, 55, &[], &devices).is_none());
    assert!(observe(&proc, &process, 55, &fifos, &devices)
        .unwrap()
        .contains("unreported"));
    symlink(dri.join("renderD128"), process_dir.join("fd/1")).unwrap();
    assert!(observe(&proc, &process, 55, &fifos, &devices)
        .unwrap()
        .contains("opened GPU: AMD"));
    process.arguments[4] = "libx264".into();
    assert!(observe(&proc, &process, 55, &fifos, &devices)
        .unwrap()
        .contains("CPU / software"));
    process.arguments[4] = "h264_nvenc".into();
    assert!(observe(&proc, &process, 55, &fifos, &devices)
        .unwrap()
        .contains("unreported"));
    process.arguments[4] = "unknown".into();
    assert!(observe(&proc, &process, 55, &fifos, &devices).is_none());
    process.arguments.clear();
    assert!(observe(&proc, &process, 55, &fifos, &devices).is_none());
    assert!(active(0, &fifos, &devices).is_empty());
    assert!(active(std::process::id(), &[], &devices).is_empty());
}

#[test]
fn t727_pci_names_are_optional_bounded_and_do_not_cross_vendor_sections() {
    let data = "1002  AMD\n\t73ff  Radeon RX 6600 XT\n\t\t1002 0001  Variant\n\n8086  Intel\n\t73ff  Other product\n";
    assert_eq!(pci_product(data, "1002:73FF"), Some("Radeon RX 6600 XT"));
    assert_eq!(pci_product(data, "8086:73FF"), Some("Other product"));
    for invalid in ["", "1002", "1002:ffff", "ffff:73ff", ":", "1002:73ff:00"] {
        assert!(pci_product(data, invalid).is_none());
    }
    assert!(
        device_label("pci-first-render", "PCI_ID=1002:73FF\nDRIVER=amdgpu", data)
            .contains("Radeon RX 6600 XT")
    );
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("pci.ids");
    assert_eq!(pci_database(&[&file]), "");
    std::fs::write(&file, data).unwrap();
    assert_eq!(pci_database(&[&root.path().join("absent"), &file]), data);
    std::fs::write(&file, vec![b'x'; 4 * 1024 * 1024 + 1]).unwrap();
    assert_eq!(pci_database(&[&file]).len(), 4 * 1024 * 1024);
}

fn discover_in(dri: &Path, sys: &Path, accessible: impl Fn(&Path) -> bool) -> Vec<Device> {
    Discovery::default().refresh_in(dri, sys, accessible, String::new)
}

#[test]
fn t1574_unchanged_devices_reuse_labels_but_refresh_permissions() {
    let (_root, dri, sys) = fixture();
    let mut discovery = Discovery::default();
    let reads = std::cell::Cell::new(0);
    for allowed in [true, false, true] {
        let devices = discovery.refresh_in(
            &dri,
            &sys,
            |_| allowed,
            || {
                reads.set(reads.get() + 1);
                "1002  AMD\n\t73ff  Test GPU\n".into()
            },
        );
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].adapter.accessible, allowed);
        assert!(devices[0].adapter.label.contains("Test GPU"));
    }
    assert_eq!(
        reads.get(),
        1,
        "T1574 unchanged inventory reread PCI database"
    );
}

#[test]
fn t1574_permissions_probe_does_not_open_render_nodes() {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;
    let (_root, dri, sys) = fixture();
    let node = std::ffi::CString::new(dri.join("renderD128").as_os_str().as_bytes()).unwrap();
    let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
    assert!(fd >= 0);
    let watch = unsafe { OwnedFd::from_raw_fd(fd) };
    assert!(
        unsafe { libc::inotify_add_watch(watch.as_raw_fd(), node.as_ptr(), libc::IN_OPEN) } >= 0
    );
    let devices = Discovery::default().refresh_in(&dri, &sys, permissions, String::new);
    assert!(devices[0].adapter.accessible);
    let mut buffer = [0u8; 1024];
    let count = unsafe { libc::read(watch.as_raw_fd(), buffer.as_mut_ptr().cast(), buffer.len()) };
    let error = std::io::Error::last_os_error();
    assert_eq!(count, -1, "T1574 discovery opened the render node");
    assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
}

#[test]
fn t1574_measure_thirty_unchanged_discovery_samples() {
    let (root, dri, sys) = fixture();
    let database = root.path().join("t1574-pci.ids");
    let mut text = "1002  AMD\n\t73ff  Test GPU\n".to_owned();
    text.push_str(&"# bounded fixture padding\n".repeat(65536));
    std::fs::write(&database, &text).unwrap();
    let mut discovery = Discovery::default();
    let began = std::time::Instant::now();
    for _ in 0..30 {
        let devices = discovery.refresh_in(&dri, &sys, permissions, || pci_database(&[&database]));
        assert_eq!(devices.len(), 1);
        assert!(devices[0].adapter.label.contains("Test GPU"));
    }
    println!(
        "T1574 fixture_queries=30 database_bytes={} elapsed_us={}",
        text.len(),
        began.elapsed().as_micros()
    );
}

#[test]
fn t1574_changed_identity_and_empty_inventory_refresh_cached_labels() {
    let (_root, dri, sys) = fixture();
    let mut discovery = Discovery::default();
    let reads = std::cell::Cell::new(0);
    let mut refresh = || {
        discovery.refresh_in(
            &dri,
            &sys,
            |_| true,
            || {
                reads.set(reads.get() + 1);
                "1002  AMD\n\t73ff  First GPU\n8086  Intel\n\t1234  Second GPU\n".into()
            },
        )
    };
    assert!(refresh()[0].adapter.label.contains("First GPU"));
    std::fs::write(
        sys.join("renderD128/device/uevent"),
        "PCI_ID=8086:1234\nDRIVER=other\n",
    )
    .unwrap();
    assert!(refresh()[0].adapter.label.contains("Second GPU · other"));
    assert_eq!(reads.get(), 2);
    assert!(refresh()[0].adapter.label.contains("Second GPU"));
    assert_eq!(reads.get(), 2);
    let alias = dri.join("by-path/pci-first-render");
    std::fs::remove_file(&alias).unwrap();
    assert!(refresh().is_empty());
    assert_eq!(reads.get(), 2);
    symlink("../renderD128", &alias).unwrap();
    assert_eq!(refresh().len(), 1);
    assert_eq!(reads.get(), 3);
    std::fs::remove_dir_all(dri.join("by-path")).unwrap();
    assert!(refresh().is_empty());
    assert_eq!(reads.get(), 3);
}

#[test]
fn t1574_device_alias_retarget_cannot_reuse_old_metadata() {
    let (_root, dri, sys) = fixture();
    let mut discovery = Discovery::default();
    let first = discovery.refresh_in(&dri, &sys, |_| true, String::new);
    let alias = dri.join("by-path/pci-first-render");
    std::fs::create_dir_all(sys.join("renderD129/device")).unwrap();
    std::fs::write(dri.join("renderD129"), "").unwrap();
    std::fs::write(sys.join("renderD129/device/uevent"), "PCI_ID=8086:1234\n").unwrap();
    std::fs::remove_file(&alias).unwrap();
    symlink("../renderD129", &alias).unwrap();
    let changed = discovery.refresh_in(&dri, &sys, |_| false, String::new);
    assert_eq!(changed[0].adapter.id, first[0].adapter.id);
    assert_ne!(changed[0].node, first[0].node);
    assert!(changed[0].adapter.label.starts_with("Intel "));
    assert!(!changed[0].adapter.accessible);
}

#[test]
fn t1574_cache_bounds_devices_metadata_and_display_strings() {
    let (_root, dri, sys) = fixture();
    for index in 0..300 {
        symlink(
            "../renderD128",
            dri.join("by-path").join(format!("test-{index}-render")),
        )
        .unwrap();
    }
    std::fs::write(
        sys.join("renderD128/device/uevent"),
        format!("PCI_ID=1002:73FF\nDRIVER={}\n", "x".repeat(8192)),
    )
    .unwrap();
    let database = format!("1002  AMD\n\t73ff  {}\n", "ø".repeat(4096));
    let devices = Discovery::default().refresh_in(&dri, &sys, |_| true, || database);
    assert_eq!(devices.len(), 256);
    assert!(devices
        .iter()
        .all(|device| device.adapter.label.chars().count() <= 600));
    assert!(devices
        .windows(2)
        .all(|pair| pair[0].adapter.id < pair[1].adapter.id));
}

#[test]
fn t1574_permissions_reject_invalid_and_missing_paths() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let node = root.path().join("render");
    assert!(!permissions(&node));
    assert!(!permissions(Path::new("bad\0path")));
    std::fs::write(&node, "").unwrap();
    std::fs::set_permissions(&node, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(permissions(&node));
    std::fs::set_permissions(&node, std::fs::Permissions::from_mode(0)).unwrap();
    assert_eq!(permissions(&node), unsafe { libc::geteuid() } == 0);
}
