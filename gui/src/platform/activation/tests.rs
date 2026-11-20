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
