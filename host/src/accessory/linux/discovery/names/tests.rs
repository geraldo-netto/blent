use super::*;
use crate::accessory::linux::{discovery::Support, Target};

fn fixture() -> (tempfile::TempDir, Candidate) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("3-2.4.4");
    std::fs::create_dir(&path).unwrap();
    for (name, value) in [
        ("idVendor", "18d1\n"),
        ("idProduct", "2d01\n"),
        ("devnum", "7\n"),
        ("manufacturer", "Unisoc\n"),
        ("product", "RugKing Pad 2 Pro\n"),
    ] {
        std::fs::write(path.join(name), value).unwrap();
    }
    (
        root,
        Candidate {
            target: Target::new(3, &[2, 4, 4]).unwrap(),
            address: 7,
            vendor: 0x18d1,
            product: 0x2d01,
            device_class: 0,
            support: Support::Accessory,
            names: DeviceNames::default(),
            availability: blent_config::connection::control::Availability::Available,
        },
    )
}

#[test]
fn t1336_cached_model_names_require_matching_observation() {
    let (root, mut candidate) = fixture();
    let names = read(root.path(), &candidate);
    assert_eq!(names.manufacturer(), "Unisoc");
    assert_eq!(names.model(), "RugKing Pad 2 Pro");
    candidate.address = 8;
    assert_eq!(read(root.path(), &candidate), DeviceNames::default());
    candidate.address = 7;
    candidate.vendor = 0;
    assert_eq!(read(root.path(), &candidate), DeviceNames::default());
    candidate.vendor = 0x18d1;
    candidate.product = 0;
    assert_eq!(read(root.path(), &candidate), DeviceNames::default());
}

#[test]
fn t1336_missing_names_do_not_hide_device_or_other_name() {
    let (root, candidate) = fixture();
    let path = root.path().join("3-2.4.4");
    std::fs::remove_file(path.join("manufacturer")).unwrap();
    let names = read(root.path(), &candidate);
    assert_eq!(names.manufacturer(), "");
    assert_eq!(names.model(), "RugKing Pad 2 Pro");
    std::fs::remove_file(path.join("product")).unwrap();
    assert_eq!(read(root.path(), &candidate), DeviceNames::default());
    std::fs::remove_dir_all(&path).unwrap();
    assert_eq!(read(root.path(), &candidate), DeviceNames::default());
}

#[test]
fn t1336_descriptor_bytes_are_bounded_and_safe_for_display() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("attribute");
    for invalid in [
        vec![0xff],
        b"name\0model".to_vec(),
        "name\u{202e}model".as_bytes().to_vec(),
        vec![b'x'; 258],
        vec![b'x'; 257],
    ] {
        std::fs::write(&path, invalid).unwrap();
        assert_eq!(attribute(&path), None);
    }
    for value in ["", "   \n", "Model\n", "平板", &"x".repeat(256)] {
        std::fs::write(&path, value).unwrap();
        assert_eq!(attribute(&path).as_deref(), Some(value.trim()));
    }
    assert_eq!(attribute(root.path()), None);
}

#[tokio::test]
async fn t1336_unplug_during_names_read_discards_hints() {
    use std::os::unix::ffi::OsStrExt;
    let (root, candidate) = fixture();
    let path = root.path().join("3-2.4.4");
    std::fs::remove_file(path.join("manufacturer")).unwrap();
    let fifo = std::ffi::CString::new(path.join("manufacturer").as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let writer = tokio::task::spawn_blocking(move || {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(path.join("manufacturer"))
            .unwrap();
        std::fs::write(path.join("devnum"), "8\n").unwrap();
        file.write_all(b"Unisoc\n").unwrap();
    });
    assert_eq!(read(root.path(), &candidate), DeviceNames::default());
    writer.await.unwrap();
}

#[tokio::test]
async fn t1336_cancelled_descriptor_batch_retains_native_admission() {
    use std::os::unix::ffi::OsStrExt;
    let (root, candidate) = fixture();
    let path = root.path().join("3-2.4.4/manufacturer");
    std::fs::remove_file(&path).unwrap();
    let fifo = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let (entered, ready) = oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let writer = Task::spawn_blocking(move || {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().write(true).open(path)?;
        let _ = entered.send(());
        let _ = wait.recv();
        file.write_all(b"Unisoc\n")?;
        Ok(())
    })
    .unwrap();
    let (worker, result) = start(root.path().to_owned(), vec![candidate]).unwrap();
    let completion = worker.completion();
    ready.await.unwrap();
    drop(worker);
    drop(result);
    assert!(
        !completion.is_complete(),
        "T1336 native read still owns admission"
    );
    release.send(()).unwrap();
    writer.await.unwrap().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !completion.is_complete() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t1336_named_batch_publishes_only_after_worker_join() {
    let (root, candidate) = fixture();
    let (worker, result) = start(root.path().to_owned(), vec![candidate]).unwrap();
    let completion = worker.completion();
    worker.await.unwrap().unwrap();
    assert!(completion.is_complete());
    let candidates = result.await.unwrap();
    assert_eq!(candidates[0].names.model(), "RugKing Pad 2 Pro");
}
