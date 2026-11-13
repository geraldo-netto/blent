use super::*;
use crate::activity::tests::{private, Fixture};
#[test]
fn t1008_native_mapping_rejects_unknown_desktop_names() {
    for desktop in [
        "Cinnamon",
        "X-Cinnamon",
        "GNOME:Cinnamon",
        "cinnamon",
        "GNOME",
        "gnome",
    ] {
        assert!(Target::desktop(desktop).is_some());
    }
    for desktop in ["", "KDE", "GNOMEExtra", "not-Cinnamon"] {
        assert!(Target::desktop(desktop).is_none());
    }
}
#[test]
fn t1008_native_mode_fuzz_rejects_invalid_values_and_ranges() {
    let mut random = 0xf187_a214u32;
    for _ in 0..1024 {
        random ^= random << 13;
        random ^= random >> 17;
        random ^= random << 5;
        let value = random as i32;
        assert_eq!(display_mode(value).is_ok(), (-1..=3).contains(&value));
    }
    for mode in [i32::MIN, -2, -1, 0, 1, 2, 3, 4, i32::MAX] {
        let result = display_mode(mode);
        match mode {
            -1 => assert_eq!(result.unwrap(), None),
            0 => assert_eq!(result.unwrap(), Some(true)),
            1..=3 => assert_eq!(result.unwrap(), Some(false)),
            _ => assert!(result.is_err()),
        }
    }
}
#[test]
fn t1008_native_lost_fields_preserve_sibling_authority_and_power_flags() {
    let running = HostState {
        power: Some(blent_config::host_activity::Power::Running),
        locked: Some(false),
        display_on: Some(true),
    };
    for mask in 0..8 {
        let caps = Capabilities::from_mask(mask).unwrap();
        let lost = Patch::Lost(caps).apply(running);
        assert_eq!(lost.power.is_none(), caps.power);
        assert_eq!(lost.locked.is_none(), caps.lock);
        assert_eq!(lost.display_on.is_none(), caps.display);
    }
    for sleep in [None, Some(false), Some(true)] {
        for shutdown in [None, Some(false), Some(true)] {
            let data = Logind {
                session: None,
                sleep: Flag {
                    value: sleep,
                    ..Flag::default()
                },
                shutdown: Flag {
                    value: shutdown,
                    ..Flag::default()
                },
                locked: Flag::default(),
            };
            let power = data.power();
            assert_eq!(
                power == Some(blent_config::host_activity::Power::Running),
                sleep == Some(false) && shutdown == Some(false)
            );
        }
    }
}
fn message(
    path: &str,
    interface: &str,
    member: &str,
    body: impl serde::Serialize + zbus::zvariant::DynamicType,
) -> Message {
    Message::signal(path, interface, member)
        .unwrap()
        .build(&body)
        .unwrap()
}
#[test]
fn t1008_bounded_native_property_packets_validate_before_state_publication() {
    let path = "/org/cinnamon/Muffin/DisplayConfig";
    let interface = "org.cinnamon.Muffin.DisplayConfig";
    for count in [0, 1, 64, 65, 128] {
        let values: HashMap<String, OwnedValue> = (0..count)
            .map(|index| (format!("p{index}"), OwnedValue::from(true)))
            .collect();
        let packet = message(
            path,
            PROPERTIES,
            "PropertiesChanged",
            (interface, values, Vec::<String>::new()),
        );
        assert_eq!(
            changed(&packet, interface, "PowerSaveMode").is_ok(),
            count <= 64
        );
    }
    let oversized = message(path, PROPERTIES, "PropertiesChanged", "x".repeat(8193));
    assert!(changed(&oversized, interface, "PowerSaveMode").is_err());
    let packet = message(path, PROPERTIES, "WrongMember", ());
    assert!(changed(&packet, interface, "PowerSaveMode")
        .unwrap()
        .is_none());
    let packet = message(
        path,
        PROPERTIES,
        "PropertiesChanged",
        (
            "other.Interface",
            HashMap::<String, OwnedValue>::new(),
            Vec::<String>::new(),
        ),
    );
    assert!(changed(&packet, interface, "PowerSaveMode")
        .unwrap()
        .is_none());
    let packet = message(
        path,
        PROPERTIES,
        "PropertiesChanged",
        (
            interface,
            HashMap::<String, OwnedValue>::new(),
            vec!["PowerSaveMode"],
        ),
    );
    assert!(matches!(
        changed(&packet, interface, "PowerSaveMode").unwrap(),
        Some(None)
    ));
    let packet = message(
        path,
        PROPERTIES,
        "PropertiesChanged",
        (
            interface,
            HashMap::from([("PowerSaveMode", OwnedValue::from(0i32))]),
            Vec::<String>::new(),
        ),
    );
    assert!(matches!(
        changed(&packet, interface, "PowerSaveMode").unwrap(),
        Some(Some(_))
    ));
    let packet = message(path, PROPERTIES, "PropertiesChanged", (3u32,));
    assert!(changed(&packet, interface, "PowerSaveMode").is_err());
}
#[tokio::test]
async fn t1008_private_original_signal_fences_reject_replay_foreign_and_malformed_state() {
    if !private("activity::native::tests::t1008_private_original_signal_fences_reject_replay_foreign_and_malformed_state") { return; }
    let fixture = Fixture::new().await;
    let bus = Connection::session().await.unwrap();
    let mut actor = Actor::logind(bus.clone(), None, 32, Duration::from_secs(2))
        .await
        .unwrap();
    fixture
        .server
        .emit_signal(
            None::<&str>,
            LOGIN_PATH,
            MANAGER,
            "PrepareForSleep",
            &(true,),
        )
        .await
        .unwrap();
    let packet = tokio::time::timeout(Duration::from_secs(2), actor.signals.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(
        actor.signal(&packet).unwrap(),
        Some(Patch::Power(Some(
            blent_config::host_activity::Power::Suspending
        )))
    ));
    assert!(actor.signal(&packet).unwrap().is_none());
    let foreign = message(LOGIN_PATH, MANAGER, "PrepareForSleep", (false,));
    assert!(actor.signal(&foreign).unwrap().is_none());
    let wrong = message(LOGIN_PATH, MANAGER, "UnknownMember", ());
    if let Kind::Logind(data) = &mut actor.kind {
        assert!(data.signal(&wrong).unwrap().is_none());
    }
    let foreign_path = message(
        "/org/freedesktop/login1/session/other",
        PROPERTIES,
        "PropertiesChanged",
        (),
    );
    if let Kind::Logind(data) = &mut actor.kind {
        assert!(data.signal(&foreign_path).unwrap().is_none());
    }
    let bad = message(LOGIN_PATH, MANAGER, "PrepareForSleep", (17u32,));
    if let Kind::Logind(data) = &mut actor.kind {
        assert!(data.signal(&bad).is_err());
    }
    let same_owner = message(
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "NameOwnerChanged",
        (LOGIN, actor.owner.clone(), actor.owner.clone()),
    );
    assert!(actor.owner_change(same_owner).is_ok());
    let changed_owner = message(
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "NameOwnerChanged",
        (LOGIN, actor.owner.clone(), ":1.999"),
    );
    assert!(actor.owner_change(changed_owner).is_err());
    assert!(boolean(&bus, &actor.owner, LOGIN_PATH, MANAGER, "Missing")
        .await
        .value
        .is_none());
    assert!(boolean(
        &bus,
        &actor.owner,
        "/org/cinnamon/Muffin/DisplayConfig",
        "org.cinnamon.Muffin.DisplayConfig",
        "PowerSaveMode"
    )
    .await
    .value
    .is_none());
    let target = Target::desktop("Cinnamon").unwrap();
    let mut display = Actor::display(
        Connection::session().await.unwrap(),
        target,
        32,
        Duration::from_secs(2),
    )
    .await
    .unwrap();
    let wrong = message("/wrong", PROPERTIES, "PropertiesChanged", ());
    if let Kind::Display { state, .. } = &mut display.kind {
        assert!(display_signal(target, state, &wrong).unwrap().is_none());
    }
    let wrong = message(target.path, "other.Interface", "PropertiesChanged", ());
    if let Kind::Display { state, .. } = &mut display.kind {
        assert!(display_signal(target, state, &wrong).unwrap().is_none());
    }
    let absent = message(
        target.path,
        PROPERTIES,
        "PropertiesChanged",
        (
            target.name,
            HashMap::<String, OwnedValue>::new(),
            Vec::<String>::new(),
        ),
    );
    if let Kind::Display { state, .. } = &mut display.kind {
        assert!(display_signal(target, state, &absent).unwrap().is_none());
    }
}

#[test]
fn t1429_initial_query_error_classification_preserves_explicit_unsupported() {
    for (name, retry) in [
        ("UnknownProperty", false),
        ("UnknownInterface", false),
        ("UnknownMethod", false),
        ("Failed", true),
        ("NoReply", true),
        ("AccessDenied", true),
        ("UnknownObject", true),
    ] {
        let error = zbus::Error::MethodError(
            format!("org.freedesktop.DBus.Error.{name}")
                .try_into()
                .unwrap(),
            None,
            message(LOGIN_PATH, MANAGER, "Probe", ()),
        );
        assert_eq!(retry_query(&error.into()), retry);
    }
    assert!(retry_query(&anyhow::anyhow!("malformed snapshot")));
}
#[tokio::test]
async fn t1429_invalid_initial_boolean_values_request_recovery() {
    if !private("activity::native::tests::t1429_invalid_initial_boolean_values_request_recovery") {
        return;
    }
    let fixture = Fixture::new().await;
    let bus = Connection::session().await.unwrap();
    let wrong = boolean(
        &bus,
        "org.cinnamon.Muffin.DisplayConfig",
        "/org/cinnamon/Muffin/DisplayConfig",
        "org.cinnamon.Muffin.DisplayConfig",
        "PowerSaveMode",
    )
    .await;
    assert!(wrong.value.is_none());
    assert!(wrong.initial_failed);
    let absent = boolean(&bus, LOGIN, LOGIN_PATH, MANAGER, "AbsentProperty").await;
    assert!(absent.value.is_none());
    assert!(!absent.initial_failed);
    bus.close().await.unwrap();
    fixture.server.close().await.unwrap();
}

#[test]
fn t1438_session_lookup_distinguishes_absence_from_failed_evidence() {
    for (name, absent) in [
        ("org.freedesktop.login1.NoSessionForPID", true),
        ("org.freedesktop.login1.NoSuchSession", true),
        ("org.freedesktop.DBus.Error.UnknownMethod", true),
        ("org.freedesktop.DBus.Error.UnknownInterface", true),
        ("org.freedesktop.DBus.Error.Failed", false),
        ("org.freedesktop.DBus.Error.NoReply", false),
        ("org.freedesktop.DBus.Error.AccessDenied", false),
    ] {
        let error = zbus::Error::MethodError(
            name.try_into().unwrap(),
            None,
            message(LOGIN_PATH, MANAGER, "Probe", ()),
        );
        assert_eq!(session_lookup(Err(error)).is_ok(), absent);
    }
    assert!(session_lookup(Err(zbus::Error::Failure("closed connection".into()))).is_err());
}
#[test]
fn t1438_session_identity_payload_bounds_and_uid_are_validated() {
    for count in [0, 1, 225, 226, 255, 256, 257, 1024] {
        let name = format!("/org/freedesktop/login1/session/{}", "x".repeat(count));
        if let Ok(path) = OwnedObjectPath::try_from(name.clone()) {
            let reply = message(LOGIN_PATH, MANAGER, "Probe", path);
            assert_eq!(session_object(&reply).is_ok(), name.len() <= 256);
        }
    }
    for body in ["/", "/unrelated/session"] {
        let reply = message(
            LOGIN_PATH,
            MANAGER,
            "Probe",
            OwnedObjectPath::try_from(body).unwrap(),
        );
        assert!(session_object(&reply).is_err());
    }
    assert!(session_object(&message(LOGIN_PATH, MANAGER, "Probe", 42u32)).is_err());
    let path = OwnedObjectPath::try_from("/org/freedesktop/login1/session/fixture").unwrap();
    for value in [OwnedValue::from(true), OwnedValue::from(42u32)] {
        assert!(own_session(path.clone(), value).is_err());
    }
    for uid in [
        unsafe { libc::geteuid() },
        unsafe { libc::geteuid() }.wrapping_add(1),
    ] {
        let value = zbus::zvariant::Value::from((uid, path.clone()))
            .try_to_owned()
            .unwrap();
        assert_eq!(
            own_session(path.clone(), value).unwrap().is_some(),
            uid == unsafe { libc::geteuid() }
        );
    }
}
