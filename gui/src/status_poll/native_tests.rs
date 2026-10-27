use super::*;
use blent_config::{
    connection::pairing::PeerId,
    tablets::{Identity, TabletSession},
};
fn native() -> TabletSession {
    TabletSession {
        identity: Identity::UsbPeer(PeerId::parse("ab".repeat(32)).unwrap()),
        instance: 0,
        video_port: 8890,
        input_port: 8891,
    }
}
#[test]
fn t922_usb_status_does_not_require_or_probe_adb() {
    for available in [false, true] {
        let mut status = Status::default();
        linux::query_tablets(&mut status, vec![native()], available, || {
            panic!("T922 native display queried ADB")
        });
        assert!(
            !status.tablets.is_empty(),
            "T922 USB display hidden without ADB"
        );
        assert_eq!(
            status
                .tablets
                .iter()
                .map(|device| device.name())
                .collect::<Vec<_>>()
                .join(", "),
            "USB tablet (abababababab)"
        );
    }
}
#[test]
fn t922_failed_adb_inventory_preserves_independent_usb_status() {
    let mut adb = native();
    adb.instance = 1;
    adb.identity = Identity::Adb("ADB".into());
    let mut status = Status::default();
    linux::query_tablets(&mut status, vec![adb.clone(), native()], true, || {
        Err(std::io::ErrorKind::TimedOut.into())
    });
    assert!(!status.tablets.is_empty());
    assert_eq!(
        status
            .tablets
            .iter()
            .map(|device| device.name())
            .collect::<Vec<_>>()
            .join(", "),
        "USB tablet (abababababab)"
    );
    linux::query_tablets(&mut status, vec![adb, native()], true, || {
        Ok("ADB device model:Tablet_Model\n".into())
    });
    assert_eq!(
        status
            .tablets
            .iter()
            .map(|device| device.name())
            .collect::<Vec<_>>()
            .join(", "),
        "USB tablet (abababababab), Tablet Model"
    );
}
#[test]
fn t922_empty_and_adb_only_status_keep_inventory_rules() {
    let mut status = Status {
        tablets: vec![blent_config::workspace::Device::new(
            blent_config::tablets::Identity::Adb("stale".into()),
            "stale".into(),
            blent_config::workspace::Presence::Unknown,
            false,
        )
        .unwrap()],
        ..Default::default()
    };
    linux::query_tablets(&mut status, vec![], true, || {
        panic!("empty status queried ADB")
    });
    assert!(status.tablets.is_empty());
    let mut adb = native();
    adb.identity = Identity::Adb("tablet_serial".into());
    linux::query_tablets(&mut status, vec![adb.clone()], false, || {
        panic!("unavailable ADB queried")
    });
    assert!(status.tablets.is_empty());
    linux::query_tablets(&mut status, vec![adb], true, || {
        Ok(
            "List of devices attached\n\nshort\ntablet_serial offline\ntablet_serial device\n"
                .into(),
        )
    });
    assert_eq!(
        status
            .tablets
            .iter()
            .map(|device| device.name())
            .collect::<Vec<_>>()
            .join(", "),
        "tablet serial"
    );
}
