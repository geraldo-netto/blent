use super::*;

#[test]
fn t1090_retirement_policy_exhausts_creation_and_route_boundaries() {
    let cases = [
        (
            Creation::NotStarted,
            [Cleanup::Complete, Cleanup::Complete, Cleanup::Complete],
        ),
        (
            Creation::Uncertain,
            [Cleanup::Wait, Cleanup::Remove, Cleanup::Wait],
        ),
        (
            Creation::Acknowledged,
            [Cleanup::Complete, Cleanup::Remove, Cleanup::Complete],
        ),
        (
            Creation::RemovalPending,
            [Cleanup::Complete, Cleanup::Wait, Cleanup::Wait],
        ),
    ];
    for (creation, expected) in cases {
        for (route, action) in [Route::Missing, Route::Owned, Route::Foreign]
            .into_iter()
            .zip(expected)
        {
            assert_eq!(creation.cleanup(route), action);
        }
    }
}

#[test]
fn t1090_route_listing_rejects_ambiguity_and_preserves_unrelated_entries() {
    for port in [1, 1024, 12345, 65535] {
        let remote = format!("tcp:{port}");
        let owned = format!("Usb {remote} tcp:42\nUsb localabstract:unrelated tcp:77\n");
        assert_eq!(route(&owned, &remote, "tcp:42").unwrap(), Route::Owned);
        assert_eq!(route(&owned, &remote, "tcp:43").unwrap(), Route::Foreign);
        assert_eq!(route("", &remote, "tcp:42").unwrap(), Route::Missing);
        assert!(route(&format!("{owned}{owned}"), &remote, "tcp:42").is_err());
    }
    for count in 1..12 {
        assert_eq!(
            route(&"field ".repeat(count), "tcp:1", "tcp:2").is_ok(),
            count == 3
        );
    }
    assert!(route(&" ".repeat(65536), "tcp:1", "tcp:2").is_ok());
    assert!(route(&" ".repeat(65537), "tcp:1", "tcp:2").is_err());
}

#[test]
fn t1090_endpoint_port_bounds_reject_before_admission() {
    for invalid in [
        "",
        "tcp:",
        "tcp:0",
        "tcp:-1",
        "tcp:65536",
        "udp:1",
        "tcp:1;id",
    ] {
        assert!(port(invalid).is_err(), "{invalid}");
    }
    for value in [1, 1024, 65535] {
        assert_eq!(port(&format!("tcp:{value}")).unwrap(), value);
    }
}
