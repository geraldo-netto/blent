use super::*;

fn name() -> Vec<u8> {
    tempfile::tempdir()
        .unwrap()
        .path()
        .as_os_str()
        .as_encoded_bytes()
        .to_vec()
}

#[test]
fn t1449_repeated_launch_keeps_original_state_owner() {
    let name = name();
    let launches = std::cell::Cell::new(0);
    run_at(&name, |_original| {
        launches.set(launches.get() + 1);
        run_at(&name, |_| {
            launches.set(launches.get() + 1);
            Ok(())
        })
    })
    .unwrap();
    assert_eq!(
        launches.get(),
        1,
        "T1449 second launch created independent GUI state"
    );
}

#[test]
fn t1449_flood_and_invalid_messages_are_bounded() {
    let name = name();
    run_at(&name, |instance| {
        let address = SocketAddr::from_abstract_name(&name).unwrap();
        let sender = UnixDatagram::unbound().unwrap();
        for message in [b"".as_slice(), b"SHOW", b"show!", b"show trailing"] {
            sender.send_to_addr(message, &address).unwrap();
            assert!(!instance.requested().unwrap());
        }
        for _ in 0..128 {
            run_at(&name, |_| panic!("duplicate"))?;
        }
        assert!(instance.requested().unwrap());
        for _ in 0..4 {
            instance.requested().unwrap();
        }
        assert!(!instance.requested().unwrap());
        Ok(())
    })
    .unwrap();
}

#[test]
fn t1449_error_unwind_and_repeated_reuse_release_descriptors() {
    let name = name();
    let count = || std::fs::read_dir("/proc/self/fd").unwrap().count();
    let before = count();
    for _ in 0..64 {
        assert!(run_at(&name, |_| Err(error(io::Error::other("fixture")))).is_err());
        let result = std::panic::catch_unwind(|| run_at(&name, |_| panic!("fixture")));
        assert!(result.is_err());
        run_at(&name, |_| Ok(())).unwrap();
    }
    assert_eq!(count(), before, "T1449 leaked socket descriptors");
    assert!(run_at(&[b'x'; 200], |_| panic!("invalid name admitted")).is_err());
    let absent = SocketAddr::from_abstract_name(name).unwrap();
    assert!(notify(&absent).is_err());
}

#[test]
fn t1449_cross_process_activation_has_no_new_state_owner() {
    let root = tempfile::tempdir().unwrap();
    let name = root.path().to_str().unwrap();
    run_at(name.as_bytes(), |instance| {
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "platform::activation::tests::t1449_secondary_fixture",
            ])
            .env("BLENT_T1449_NAME", name)
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{}",
            String::from_utf8_lossy(&child.stdout)
        );
        assert!(instance.requested().unwrap());
        Ok(())
    })
    .unwrap();
}

#[test]
fn t1449_secondary_fixture() {
    if let Ok(name) = std::env::var("BLENT_T1449_NAME") {
        run_at(name.as_bytes(), |_| {
            panic!("T1449 secondary created new owner")
        })
        .unwrap();
    }
}

#[test]
fn t1454_long_runtime_entry_preserves_single_owner() {
    let root = tempfile::tempdir().unwrap();
    let runtime = root.path().join("r".repeat(180));
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&runtime)
        .unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "platform::activation::tests::t1454_runtime_child",
            "--nocapture",
        ])
        .env("BLENT_T1454_CHILD", "1")
        .env("XDG_RUNTIME_DIR", &runtime)
        .env("HOME", root.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "T1454: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn t1454_runtime_child() {
    if std::env::var_os("BLENT_T1454_CHILD").is_none() {
        return;
    }
    run(|original| {
        run(|_| panic!("T1454 duplicate owner"))?;
        assert!(original.requested().unwrap());
        Ok(())
    })
    .unwrap();
    run(|_| Ok(())).unwrap();
}

#[test]
fn t1454_long_home_paths_and_suffixes_remain_distinct() {
    let root = tempfile::tempdir().unwrap();
    for length in [0, 1, 107, 108, 255, 4096] {
        let home = root.path().join("h".repeat(length)).join(".cache/blent");
        let first = endpoint(&home);
        assert_eq!(first.len(), 42);
        assert_eq!(first, endpoint(&home));
        let other = endpoint(&home.join("other"));
        assert_ne!(first, other);
        run_at(&first, |original| {
            run_at(&other, |_| Ok(()))?;
            assert!(!original.requested().unwrap());
            run_at(&first, |_| panic!("T1454 duplicate"))?;
            assert!(original.requested().unwrap());
            Ok(())
        })
        .unwrap();
    }
}
