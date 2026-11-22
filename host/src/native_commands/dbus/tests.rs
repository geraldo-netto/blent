use super::*;

fn call() -> Message {
    Message::method_call("/fixture", "Set")
        .unwrap()
        .sender(":1.7")
        .unwrap()
        .build(&())
        .unwrap()
}
fn error(name: &str, sender: &str) -> zbus::Error {
    let message = Message::error(&call().header(), name)
        .unwrap()
        .sender(sender)
        .unwrap()
        .build(&"denied")
        .unwrap();
    zbus::Error::MethodError(name.try_into().unwrap(), Some("denied".into()), message)
}

#[test]
fn t1640_original_reply_and_broker_predispatch_rejection_settle() {
    for sender_name in [":1.10", ":1.11"] {
        let reply = Message::method_return(&call().header())
            .unwrap()
            .sender(sender_name)
            .unwrap()
            .build(&())
            .unwrap();
        let report = classify(Ok(reply), ":1.10");
        assert_eq!(report.settled, sender_name == ":1.10");
        assert_eq!(report.result.is_ok(), sender_name == ":1.10");
    }
    for name in [
        "org.freedesktop.DBus.Error.Failed",
        "org.freedesktop.DBus.Error.NoReply",
    ] {
        let report = classify(Err(error(name, ":1.10")), ":1.10");
        assert!(report.settled);
        assert!(report.result.is_err());
    }
    for name in ["ServiceUnknown", "NameHasNoOwner", "AccessDenied"] {
        let report = classify(
            Err(error(
                &format!("org.freedesktop.DBus.Error.{name}"),
                "org.freedesktop.DBus",
            )),
            ":1.10",
        );
        assert!(report.settled);
    }
}

#[test]
fn t1640_lost_or_foreign_reply_is_not_a_settlement() {
    for sender_name in [":1.11", "org.freedesktop.DBus"] {
        for name in ["NoReply", "Disconnected", "Timeout", "Failed"] {
            let report = classify(
                Err(error(
                    &format!("org.freedesktop.DBus.Error.{name}"),
                    sender_name,
                )),
                ":1.10",
            );
            assert!(!report.settled);
            assert!(report.result.is_err());
        }
    }
    assert!(
        !classify(
            Err(zbus::Error::InputOutput(
                io::Error::from(io::ErrorKind::BrokenPipe).into()
            )),
            ":1.10"
        )
        .settled
    );
    let reply = Message::method_return(&call().header()).unwrap().build(&()).unwrap();
    assert!(!classify(Ok(reply), ":1.10").settled);
}

#[test]
fn t1640_property_admission_and_names_are_bounded() {
    for length in [0, 1, 4095, 4096, 4097] {
        let value = "x".repeat(length);
        assert_eq!(
            Property::new("/fixture", "org.blent.Fixture", "mode", "s", &value).is_ok(),
            length <= 4096
        );
        assert_eq!(
            Property::new(&value, "org.blent.Fixture", "mode", "s", "x").is_ok(),
            length <= 4096
        );
    }
    for value in ["-2147483648", "2147483647", "0"] {
        assert!(Property::new("/a", "org.blent.Fixture", "mode", "i", value).is_ok());
    }
    for value in ["-2147483649", "2147483648", "", "1.0", "true"] {
        assert!(Property::new("/a", "org.blent.Fixture", "mode", "i", value).is_err());
    }
    for signature in ["", "x", "ss"] {
        assert!(Property::new("/a", "org.blent.Fixture", "mode", signature, "1").is_err());
    }
}

#[tokio::test]
async fn t1640_invalid_names_and_absent_service_have_no_mutation_debt() {
    if !crate::activity::private_bus_test("native_commands::dbus::tests::t1640_invalid_names_and_absent_service_have_no_mutation_debt") { return; }
    for (path, interface, name) in [
        ("invalid", "org.blent.Fixture", "mode"),
        ("/a", "invalid", "mode"),
        ("/a", "org.blent.Fixture", "bad.name"),
        ("/a", "org.blent.Fixture", "mode"),
    ] {
        let result = execute(Property::new(path, interface, name, "i", "1").unwrap()).await;
        assert!(result.settled);
        assert!(result.result.is_err());
    }
}

#[tokio::test]
async fn t1640_unreachable_bus_before_dispatch_settles() {
    if !crate::activity::private_bus_test(
        "native_commands::dbus::tests::t1640_unreachable_bus_before_dispatch_settles",
    ) {
        return;
    }
    std::env::set_var("DBUS_SESSION_BUS_ADDRESS", "invalid-address");
    let report = execute(Property::new("/a", "org.blent.Fixture", "mode", "i", "1").unwrap()).await;
    assert!(report.settled);
    assert!(report.result.is_err());
}
