use super::*;

#[tokio::test]
async fn t278_wifi_setup_never_uses_mdns_as_the_usb_prerequisite() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let adb = root.path().join("adb");
    std::fs::write(&adb, "#!/bin/sh\nprintf '%s\\n' \"$2\" >> \"$0.log\"\necho package:/data/app/com.blent/base.apk\n").unwrap();
    std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o700)).unwrap();
    let network = [
        "adb-TABLET-nonce._adb-tls-connect._tcp",
        "tablet._adb._tcp.local",
        "[2001:db8::1]:5555",
    ]
    .map(String::from);
    assert_eq!(
        usb_tablet(&Adb(crate::usb::NativeCommands(adb.clone())), &network).await,
        None
    );
    assert!(
        !adb.with_extension("log").exists(),
        "T278: wireless transport probed as USB"
    );
    let mut both = network.to_vec();
    both.push("USB_TABLET".into());
    assert_eq!(
        usb_tablet(&Adb(crate::usb::NativeCommands(adb.clone())), &both)
            .await
            .as_deref(),
        Some("USB_TABLET")
    );
    assert_eq!(
        std::fs::read_to_string(adb.with_extension("log")).unwrap(),
        "USB_TABLET\n"
    );
}

#[tokio::test]
async fn t142_wifi_setup_requires_an_installed_usb_tablet() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("adb");
    std::fs::write(&path, "#!/bin/sh\ncase \"$2\" in TABLET*|192.0.2.1:5555) echo package:/data/app/com.blent/base.apk;; esac\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let adb = Adb(crate::usb::NativeCommands(path));
    assert_eq!(
        usb_tablet(&adb, &["PHONE".into(), "192.0.2.1:5555".into()]).await,
        None
    );
    assert_eq!(
        usb_tablet(
            &adb,
            &["PHONE".into(), "TABLET_A".into(), "TABLET_B".into()]
        )
        .await
        .as_deref(),
        Some("TABLET_A")
    );
    assert_eq!(usb_tablet(&adb, &["PHONE".into()]).await, None);
}
