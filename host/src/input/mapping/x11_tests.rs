//! T577: Xorg provider names are not DRM connector identities.
use super::*;
use crate::vdisplay::EvdiConnector;

pub(crate) fn edid(width: u32) -> Vec<u8> {
    let mut bytes = vec![0u8; 128];
    bytes[..8].copy_from_slice(&[0, 255, 255, 255, 255, 255, 255, 0]);
    bytes[12..16].copy_from_slice(&width.to_le_bytes());
    bytes[127] = 0u8.wrapping_sub(
        bytes[..127]
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte)),
    );
    bytes
}

pub(crate) fn output(name: &str, primary: bool, bytes: &[u8]) -> String {
    let mut text = format!(
        "{name} connected {}1280x800+0+0\n\tEDID:\n",
        if primary { "primary " } else { "" }
    );
    for row in bytes.chunks(16) {
        text.push_str("\t\t");
        for byte in row {
            text.push_str(&format!("{byte:02x}"));
        }
        text.push('\n');
    }
    text
}

fn connector(card: u32, name: &str, width: u32) -> EvdiConnector {
    EvdiConnector {
        card,
        name: name.into(),
        connected: true,
        edid: edid(width),
    }
}

#[test]
fn t577_provider_renaming_cannot_assign_another_tablets_output() {
    let connectors = [connector(1, "DVI-I-1", 1280), connector(2, "DVI-I-2", 1200)];
    let text = output("DVI-I-2-1", false, &connectors[0].edid)
        + &output("DVI-I-3-2", false, &connectors[1].edid);
    let active = x11_active_outputs(&text);
    assert_eq!(
        x11_target_output(false, &active, &connectors, Some(1)),
        Some("DVI-I-2-1")
    );
    assert_eq!(
        x11_target_output(false, &active, &connectors, Some(2)),
        Some("DVI-I-3-2")
    );
}

#[test]
fn t577_missing_invalid_and_ambiguous_identity_never_falls_back_to_names() {
    let connectors = [connector(1, "DVI-I-1", 1280)];
    for text in [
        "DVI-I-1 connected 1280x800+0+0\n".to_owned(),
        output("DVI-I-1", false, &[0; 128]),
        output("DVI-I-1", false, &edid(1200)),
        "DVI-I-1 disconnected\n".into(),
        "DVI-I-1 connected\n".into(),
    ] {
        assert!(
            x11_target_output(false, &x11_active_outputs(&text), &connectors, Some(1)).is_none()
        );
    }
    let text = output("DVI-I-2-1", false, &edid(1280));
    let active = x11_active_outputs(&text);
    assert!(x11_target_output(false, &active, &connectors, Some(2)).is_none());
    assert_eq!(
        x11_target_output(false, &active, &connectors, None),
        Some("DVI-I-2-1")
    );
    let same = [connector(1, "DVI-I-1", 1280), connector(2, "DVI-I-2", 1280)];
    assert!(x11_target_output(false, &active, &same, Some(1)).is_none());
    let duplicates = text.clone() + &output("DVI-I-3-2", false, &edid(1280));
    assert!(x11_target_output(
        false,
        &x11_active_outputs(&duplicates),
        &connectors,
        Some(1)
    )
    .is_none());
    let mut disconnected = connectors;
    disconnected[0].connected = false;
    assert!(x11_target_output(false, &active, &disconnected, Some(1)).is_none());
}

#[test]
fn t577_physical_mode_excludes_renamed_virtual_outputs_and_unknown_owners() {
    let mut connectors = [connector(1, "DVI-I-1", 1280)];
    let text = output("DVI-I-2-1", true, &edid(1280))
        + &output("HDMI-A-1", false, &edid(1920))
        + &output("eDP-1", true, &edid(1600));
    let active = x11_active_outputs(&text);
    assert_eq!(
        x11_target_output(true, &active, &connectors, None),
        Some("eDP-1")
    );
    connectors[0].edid.clear();
    assert!(x11_target_output(true, &active, &connectors, None).is_none());
    assert!(x11_target_output(true, &[], &[], None).is_none());
}

#[test]
fn t577_edid_parser_rejects_bounded_invalid_lengths_and_checksums() {
    let connectors = [connector(1, "DVI-I-1", 1280)];
    for length in 0..=640 {
        let bytes = vec![0xff; length];
        let text = output("DVI-I-2-1", false, &bytes);
        assert!(
            x11_target_output(false, &x11_active_outputs(&text), &connectors, Some(1)).is_none()
        );
    }
    for offset in 0..128 {
        let mut bytes = edid(1280);
        bytes[offset] ^= 1;
        let text = output("DVI-I-2-1", false, &bytes);
        assert!(
            x11_target_output(false, &x11_active_outputs(&text), &connectors, Some(1)).is_none()
        );
    }
    for marker in [
        "EDID:\n\t\tzzzz",
        "EDID:\n\t\t🦀",
        "OTHER:\n\t\t00000000000000000000000000000000",
    ] {
        let text = format!("DVI-I-2-1 connected 1280x800+0+0\n\t{marker}\n");
        assert!(
            x11_target_output(false, &x11_active_outputs(&text), &connectors, Some(1)).is_none()
        );
    }
}

fn tool(root: &std::path::Path, name: &str, body: &str) -> String {
    use std::os::unix::fs::PermissionsExt;
    let path = root.join(name);
    std::fs::write(
        &path,
        format!("#!/usr/bin/python3\nimport pathlib, sys\nroot=pathlib.Path({root:?})\n{body}\n"),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path.to_str().unwrap().into()
}

fn input_tool(root: &std::path::Path, delay: usize) -> String {
    tool(
        root,
        "xinput",
        &format!(
            r#"
if sys.argv[1] == 'list':
 p=root/'lists'; n=int(p.read_text())+1 if p.exists() else 1; p.write_text(str(n))
 if n >= {delay}:
  print('Blent Touch id=10 [slave pointer]')
  print('Blent Touch 2 id=20 [slave pointer]')
elif sys.argv[1] != 'list-props':
 with (root/'mapped').open('a') as f: f.write(' '.join(sys.argv[1:])+'\n')
"#
        ),
    )
}

#[tokio::test]
async fn t730_missing_kwin_does_not_override_x11_mapping_results() {
    const NAME: &str =
        "input::mapping::x11_tests::t730_missing_kwin_does_not_override_x11_mapping_results";
    if crate::test_logging::isolated(NAME) {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    std::env::set_var("PATH", root.path());
    assert!(crate::kwin::backend().await.is_none());
    let owned = connector(1, "DVI-I-1", 1280);
    std::fs::write(
        root.path().join("outputs"),
        output("DVI-I-2-1", false, &owned.edid),
    )
    .unwrap();
    let randr = tool(root.path(), "xrandr", "print((root/'outputs').read_text())");
    map_devices_using(
        &crate::native_commands::AcknowledgedCommands,
        false,
        &DeviceIdentity::for_instance(0).unwrap(),
        Some(1),
        1,
        "x11",
        &input_tool(root.path(), 1),
        &randr,
        Some(&[owned]),
    )
    .await;
    assert_eq!(
        std::fs::read_to_string(root.path().join("mapped")).unwrap(),
        "map-to-output Blent Touch DVI-I-2-1\n"
    );
    let log = crate::test_logging::text();
    assert!(log.contains("Mapped 'Blent Touch'"), "{log}");
    assert!(
        !log.contains("will address the whole desktop"),
        "T730: {log}"
    );
    assert!(
        log.contains("KWin input mapping and keyboard suppression are unavailable"),
        "{log}"
    );
}

#[tokio::test]
async fn t730_unavailable_x11_mapper_reports_its_own_failure() {
    const NAME: &str =
        "input::mapping::x11_tests::t730_unavailable_x11_mapper_reports_its_own_failure";
    if crate::test_logging::isolated(NAME) {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    std::env::set_var("PATH", root.path());
    assert!(crate::kwin::backend().await.is_none());
    map_devices_using(
        &crate::native_commands::AcknowledgedCommands,
        false,
        &DeviceIdentity::for_instance(0).unwrap(),
        Some(1),
        1,
        "x11",
        "xinput",
        "xrandr",
        Some(&[]),
    )
    .await;
    let log = crate::test_logging::text();
    assert!(log.contains("xrandr"), "T730: {log}");
    assert!(!log.contains("Mapped '"), "{log}");
    assert!(
        !log.contains("will address the whole desktop"),
        "T730: {log}"
    );
}

#[tokio::test]
async fn t622_late_input_does_not_force_stable_output_probes() {
    let root = tempfile::tempdir().unwrap();
    let owned = connector(1, "DVI-I-1", 1280);
    std::fs::write(
        root.path().join("outputs"),
        output("DVI-I-2-1", false, &owned.edid),
    )
    .unwrap();
    let randr = tool(
        root.path(),
        "xrandr",
        r#"
with (root/'queries').open('a') as f: f.write(' '.join(sys.argv[1:])+'\n')
print((root/'outputs').read_text())
"#,
    );
    map_x11_devices(
        &crate::native_commands::AcknowledgedCommands,
        false,
        &DeviceIdentity::for_instance(0).unwrap(),
        Some(1),
        1,
        &input_tool(root.path(), 3),
        &randr,
        Some(&[owned]),
    )
    .await;
    let queries = std::fs::read_to_string(root.path().join("queries")).unwrap();
    assert_eq!(
        queries.lines().collect::<Vec<_>>(),
        vec!["--current --prop"; 3],
        "T622 waiting for input must not re-probe stable connectors"
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("mapped")).unwrap(),
        "map-to-output Blent Touch DVI-I-2-1\n"
    );
}

#[tokio::test]
async fn t622_stale_topology_refreshes_until_owned_edid_appears() {
    let root = tempfile::tempdir().unwrap();
    let owned = connector(1, "DVI-I-1", 1280);
    std::fs::write(
        root.path().join("wanted"),
        output("DVI-I-2-1", false, &owned.edid),
    )
    .unwrap();
    std::fs::write(
        root.path().join("foreign"),
        output("DVI-I-3-1", false, &edid(1920)),
    )
    .unwrap();
    let randr = tool(
        root.path(),
        "xrandr",
        r#"
with (root/'queries').open('a') as f: f.write(' '.join(sys.argv[1:])+'\n')
p=root/'refreshes'; n=int(p.read_text()) if p.exists() else 0
if '--current' not in sys.argv:
 n+=1; p.write_text(str(n))
print((root/('wanted' if n >= 2 else 'foreign')).read_text())
"#,
    );
    map_x11_devices(
        &crate::native_commands::AcknowledgedCommands,
        false,
        &DeviceIdentity::for_instance(0).unwrap(),
        Some(1),
        1,
        &input_tool(root.path(), 1),
        &randr,
        Some(&[owned]),
    )
    .await;
    let queries = std::fs::read_to_string(root.path().join("queries")).unwrap();
    assert_eq!(queries.lines().filter(|q| *q == "--prop").count(), 2);
    assert_eq!(
        queries.lines().filter(|q| *q == "--current --prop").count(),
        5,
        "T622 retry cached resources between explicit stale-topology refreshes"
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("mapped")).unwrap(),
        "map-to-output Blent Touch DVI-I-2-1\n"
    );
}

#[tokio::test]
async fn t625_late_pen_does_not_remap_verified_touch_and_pointer() {
    let root = tempfile::tempdir().unwrap();
    let owned = connector(1, "DVI-I-1", 1280);
    std::fs::write(
        root.path().join("outputs"),
        output("DVI-I-2-1", false, &owned.edid),
    )
    .unwrap();
    let randr = tool(root.path(), "xrandr", "print((root/'outputs').read_text())");
    let input = tool(
        root.path(),
        "xinput",
        r#"
if sys.argv[1] == 'list':
 p=root/'lists'; n=int(p.read_text())+1 if p.exists() else 1; p.write_text(str(n))
 print('Blent Touch id=10 [slave pointer]')
 print('Blent Pointer id=11 [slave pointer]')
 print('Blent Pen id=12 [slave pointer]')
 print('Blent Touch 2 id=20 [slave pointer]')
elif sys.argv[1] == 'list-props':
 print('Device Node (123): "/dev/input/event'+sys.argv[2]+'"')
 print('Coordinate Transformation Matrix (124): 1, 0, 0, 0, 1, 0, 0, 0, 1')
else:
 with (root/'mapped').open('a') as f: f.write(' '.join(sys.argv[1:])+'\n')
 if sys.argv[2]=='Blent Pen' and int((root/'lists').read_text()) < 3: sys.exit(1)
"#,
    );
    map_x11_devices(
        &crate::native_commands::AcknowledgedCommands,
        false,
        &DeviceIdentity::for_instance(0).unwrap(),
        Some(1),
        3,
        &input,
        &randr,
        Some(&[owned]),
    )
    .await;
    let mapped = std::fs::read_to_string(root.path().join("mapped")).unwrap();
    assert_eq!(
        mapped.matches("map-to-output Blent Touch ").count(),
        1,
        "T625: stable touch was remapped while waiting for pen"
    );
    assert_eq!(mapped.matches("map-to-output Blent Pointer ").count(), 1);
    assert_eq!(mapped.matches("map-to-output Blent Pen ").count(), 3);
    assert!(!mapped.contains("map-to-output Blent Touch 2 "));
}

#[test]
fn t1464_keyboard_and_unknown_classes_cannot_admit_pointer_mapping() {
    let ident = DeviceIdentity::for_instance(0).unwrap();
    for class in [
        "slave keyboard (3)",
        "master pointer (3)",
        "floating slave",
        "bogus",
        "",
    ] {
        let line = format!("    ↳ {} id=21 [{class}]", ident.pen);
        assert!(
            x11_list_entry(&line, &ident).is_none(),
            "T1464 unsupported mapping admitted: {class}"
        );
    }
    let line = format!("    ↳ {} id=20 [slave  pointer  (2)]", ident.touch);
    assert_eq!(
        x11_list_entry(&line, &ident),
        Some(("20", ident.touch.as_str(), ident.touch.as_str()))
    );
}

#[tokio::test]
async fn t1464_keyboard_rejection_does_not_quarantine_input_retirement() {
    let root = tempfile::tempdir().unwrap();
    let program = tool(
        root.path(),
        "xinput",
        r#"
if sys.argv[1] == 'map-to-output':
 with (root/'mapped').open('a') as f: f.write(sys.argv[2]+'\n')
 if sys.argv[2] == 'Blent Pen':
  print('unable to find device Blent Pen',file=sys.stderr); sys.exit(1)
"#,
    );
    let commands = crate::native_commands::ObservedCommands::default();
    let devices="Blent Touch id=20 [slave pointer (2)]\nBlent Pointer id=22 [slave pointer (2)]\nBlent Pen id=21 [slave keyboard (3)]";
    let mut cache = x11_cache::Mappings::default();
    let ident = DeviceIdentity::for_instance(0).unwrap();
    let output = x11_cache::Target {
        name: "screen".into(),
        topology: "same".into(),
    };
    assert!(!map_x11_list(&commands, devices, &ident, &program, &output, 3, &mut cache).await);
    let invoked = std::fs::read_to_string(root.path().join("mapped")).unwrap();
    assert_eq!(
        invoked, "Blent Touch\nBlent Pointer\n",
        "T1464 keyboard call creates permanent unknown mutation debt"
    );
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !commands.complete() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("T1464 original command ownership did not retire");
}

#[test]
fn t1464_incomplete_inventory_keeps_discovery_pending() {
    let ident = DeviceIdentity::for_instance(0).unwrap();
    let keyboard = "Blent Pen id=21 [slave keyboard (3)]";
    assert!(!x11_unsupported_inventory(keyboard, &ident, 3));
    assert!(x11_unsupported_inventory(keyboard, &ident, 1));
    for line in [
        "foreign id=20 [slave pointer]",
        "Blent Touch id=20 [slave pointer]",
        "Blent Pen id=bogus [slave keyboard]",
    ] {
        assert!(!x11_unsupported_inventory(line, &ident, 1));
    }
    for line in [
        "Blent Touch id=20",
        "Blent Touch id=20 [slave pointer",
        "Blent Touch id=overflow [slave pointer]",
    ] {
        assert!(x11_list_entry(line, &ident).is_none());
    }
}

#[tokio::test]
async fn t1464_unsupported_class_ends_discovery_without_mapping_success() {
    let root = tempfile::tempdir().unwrap();
    let owned = connector(1, "DVI-I-1", 1280);
    std::fs::write(
        root.path().join("outputs"),
        output("DVI-I-2-1", false, &owned.edid),
    )
    .unwrap();
    let randr = tool(root.path(), "xrandr", "print((root/'outputs').read_text())");
    let input = tool(
        root.path(),
        "xinput",
        r#"
if sys.argv[1] == 'list':
 with (root/'lists').open('a') as f: f.write('listed\n')
 print('Blent Pen id=21 [slave keyboard (3)]')
else:
 raise Exception('T1464 keyboard must never admit a pointer command')
"#,
    );
    map_x11_devices(
        &crate::native_commands::ObservedCommands::default(),
        false,
        &DeviceIdentity::for_instance(0).unwrap(),
        Some(1),
        1,
        &input,
        &randr,
        Some(&[owned]),
    )
    .await;
    assert_eq!(
        std::fs::read_to_string(root.path().join("lists")).unwrap(),
        "listed\n"
    );
}
