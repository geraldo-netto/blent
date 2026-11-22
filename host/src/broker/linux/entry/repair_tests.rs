use super::*;
use crate::secure_connection::{
    identity_store::{linux::LinuxIdentityStore, load_or_create},
    IdentityRecord,
};
use std::{
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
};

fn arguments(path: &Path, mode: &str) -> Vec<OsString> {
    vec![
        "blent-usb-broker".into(),
        "--state".into(),
        path.as_os_str().into(),
        "--reset".into(),
        mode.into(),
        "--confirm-reset".into(),
    ]
}
fn initialize(path: &Path) -> Vec<u8> {
    let mut store = LinuxIdentityStore::open(path).unwrap();
    load_or_create(&mut store, IdentityRecord::generate)
        .unwrap()
        .certificate()
        .to_vec()
}
fn damaged_trust(path: &Path) {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path.join("trust.pending"))
        .unwrap();
    file.write_all(b"interrupted").unwrap();
}

#[tokio::test]
async fn t1597_confirmed_trust_repair_preserves_identity_and_allows_broker_restart() {
    let _guard = crate::broker::linux::server::authority::test_guard().await;
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let original = initialize(root.path());
    damaged_trust(root.path());
    run(arguments(root.path(), "trust")).await.unwrap();
    assert!(!root.path().join("trust.pending").exists());
    assert_eq!(initialize(root.path()), original);
    let settings = Settings::new(
        root.path().join("runtime"),
        root.path().to_owned(),
        root.path().join("absent-helper"),
    );
    serve(settings, async { Ok(()) }).await.unwrap();
    assert_eq!(initialize(root.path()), original);
    assert!(!root.path().join("runtime/broker.sock").exists());
}

fn private_root() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap()
}
fn settings(root: &Path) -> Settings {
    Settings::new(
        root.join("runtime"),
        root.to_owned(),
        root.join("absent-helper"),
    )
}

#[tokio::test]
async fn t1597_identity_reset_requires_confirmation_and_removes_bound_trust() {
    let _guard = crate::broker::linux::server::authority::test_guard().await;
    let root = private_root();
    let original = initialize(root.path());
    damaged_trust(root.path());
    let failure = serve(settings(root.path()), std::future::pending()).await;
    assert!(failure.is_err());
    // T1813: failed bootstrap transfers its original worker into retirement.
    // This guarded fixture owns the sole shared admission; repair does not
    // imply that the native worker has joined or its capacity can be reused.
    crate::broker::linux::server::authority::test_idle().await;
    let mut unconfirmed = arguments(root.path(), "identity");
    unconfirmed.pop();
    assert!(run(unconfirmed).await.is_err());
    assert_eq!(initialize(root.path()), original);
    assert!(root.path().join("trust.pending").is_file());
    run(arguments(root.path(), "identity")).await.unwrap();
    assert!(!root.path().join("identity.bin").exists());
    assert!(!root.path().join("trust.pending").exists());
    serve(settings(root.path()), async { Ok(()) })
        .await
        .unwrap();
    assert_ne!(initialize(root.path()), original);
}

#[tokio::test]
async fn t1597_repair_refuses_live_owners_without_removing_their_state() {
    let root = private_root();
    let original = initialize(root.path());
    damaged_trust(root.path());
    let identity = LinuxIdentityStore::open(root.path()).unwrap();
    assert!(run(arguments(root.path(), "identity")).await.is_err());
    drop(identity);
    let trust = crate::secure_connection::trust_store::LinuxTrustStore::open(root.path()).unwrap();
    assert!(run(arguments(root.path(), "trust")).await.is_err());
    assert!(run(arguments(root.path(), "identity")).await.is_err());
    assert!(root.path().join("trust.pending").exists());
    assert!(!root.path().join("identity.reset").exists());
    drop(trust);
    assert_eq!(initialize(root.path()), original);
}

#[tokio::test]
async fn t1597_invalid_repair_arguments_and_paths_have_no_success_effects() {
    let root = private_root();
    damaged_trust(root.path());
    for mode in ["", "all", "IDENTITY", "trust/../identity"] {
        assert!(run(arguments(root.path(), mode)).await.is_err());
    }
    for path in [Path::new("relative"), &root.path().join("missing")] {
        assert!(run(arguments(path, "trust")).await.is_err());
    }
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(run(arguments(root.path(), "trust")).await.is_err());
    assert!(root.path().join("trust.pending").exists());
}

#[test]
fn t1597_repair_rejects_unrelated_run_options_and_missing_confirmation() {
    let root = private_root();
    for (flag, value) in [
        ("--runtime", "/tmp/unused"),
        ("--helper", "/tmp/unused"),
        ("--clients", "2"),
        ("--trust-peers", "1"),
    ] {
        let mut args = arguments(root.path(), "trust");
        args.extend([flag.into(), value.into()]);
        assert!(Options::try_parse_from(args).is_err());
    }
    let mut args = arguments(root.path(), "trust");
    args.truncate(3);
    args.push("--confirm-reset".into());
    assert!(Options::try_parse_from(args).is_err());
    let options = Options::try_parse_from(arguments(root.path(), "trust")).unwrap();
    assert!(options.settings().is_err());
    let mut options = Options::try_parse_from(arguments(root.path(), "trust")).unwrap();
    options.runtime = Some(root.path().join("runtime"));
    assert!(options.settings().is_err());
}
