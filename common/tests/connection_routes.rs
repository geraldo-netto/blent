//! T856: portable identity and local route ownership contracts.
use blent_config::connection::{pairing::PeerId, routes::*};
#[test]
fn t856_authenticated_and_observed_identity_namespaces_never_alias() {
    let peer = PeerId::parse("ab".repeat(32)).unwrap();
    let authenticated = Identity::authenticated(peer.clone());
    assert_eq!(authenticated.peer(), Some(&peer));
    assert_eq!(
        authenticated.profile(),
        Some(format!("certificate:{}", peer.as_str()).as_str())
    );
    let observed = Identity::observed("device:tablet".into(), Some("tablet".into())).unwrap();
    assert_eq!(observed.profile(), Some("tablet"));
    assert!(observed.peer().is_none());
    assert_ne!(
        authenticated,
        Identity::observed(
            peer.as_str().into(),
            authenticated.profile().map(str::to_owned)
        )
        .unwrap()
    );
    assert_eq!(Link::Usb.profile_name(), "usb");
    assert_eq!(Link::Network.profile_name(), "network");
    for value in [
        "".to_owned(),
        "x".repeat(1025),
        "nul\0id".into(),
        "line\nid".into(),
    ] {
        assert!(Identity::observed(value.clone(), None).is_err());
        assert!(Identity::observed("valid".into(), Some(value)).is_err());
    }
    assert!(Identity::observed("x".repeat(1024), None)
        .unwrap()
        .profile()
        .is_none());
}
#[test]
fn t856_route_claims_reject_collisions_until_actual_retirement() {
    let mut routes = RouteClaims::new(2).unwrap();
    let display = routes.reserve(1, &[8890, 8891]).unwrap();
    assert!(routes.current(display, 8890));
    assert!(routes.current(display, 8891));
    assert!(!routes.current(display, 8892));
    assert!(routes.reserve(2, &[8891, 9000]).is_err());
    let camera = routes.reserve(2, &[9000]).unwrap();
    assert!(routes.reserve(3, &[9001]).is_err());
    assert!(routes.retired(display).is_err());
    assert!(routes.revoke(display).unwrap());
    assert!(!routes.revoke(display).unwrap());
    assert!(!routes.current(display, 8890));
    assert!(routes.current(camera, 9000));
    assert!(routes.reserve(1, &[9001]).is_err());
    assert!(routes.reserve(3, &[8890]).is_err());
    routes.retired(display).unwrap();
    let next = routes.reserve(1, &[8890, 8891]).unwrap();
    assert!(next.revision > display.revision);
    assert_eq!(next.owner, 1);
    assert!(!routes.current(display, 8890));
    assert!(routes.revoke(display).is_err());
    assert!(routes.retired(display).is_err());
    assert!(routes.current(next, 8891));
    routes.revoke(next).unwrap();
    routes.retired(next).unwrap();
    assert!(routes.retired(next).is_err());
}
#[test]
fn t856_invalid_ports_bounds_and_failed_reservation_do_not_consume_revision() {
    for capacity in [0, 1025, usize::MAX] {
        assert!(RouteClaims::new(capacity).is_err());
    }
    let mut routes = RouteClaims::new(1024).unwrap();
    for owner in [0, u64::MAX] {
        assert!(routes.reserve(owner, &[1000]).is_err());
    }
    for ports in [
        vec![],
        vec![0],
        vec![65536],
        vec![u32::MAX],
        vec![1, 1],
        vec![1, 2, 3, 4, 5, 6],
    ] {
        assert!(routes.reserve(1, &ports).is_err());
    }
    let route = routes.reserve(1, &[1, 65535]).unwrap();
    assert_eq!(route.revision, 1);
    assert!(!routes.current(route, 0));
    assert!(!routes.current(route, u32::MAX));
    assert!(routes.reserve(1, &[1000]).is_err());
    assert!(routes.revoke(RouteId { owner: 2, ..route }).is_err());
    assert!(routes
        .revoke(RouteId {
            revision: 0,
            ..route
        })
        .is_err());
}
#[test]
fn t856_bounded_port_mutations_preserve_unrelated_owner() {
    let mut routes = RouteClaims::new(2).unwrap();
    let display = routes.reserve(1, &[8890, 8891]).unwrap();
    for value in 0..4096u32 {
        let port = value.wrapping_mul(2654435761);
        match routes.reserve(2, &[port]) {
            Ok(route) => {
                assert!((1..=65535).contains(&port));
                assert_ne!(port, 8890);
                assert_ne!(port, 8891);
                assert!(routes.current(route, port));
                routes.revoke(route).unwrap();
                routes.retired(route).unwrap();
            }
            Err(_) => assert!(port == 0 || port > 65535 || port == 8890 || port == 8891),
        }
        assert!(routes.current(display, 8890));
    }
}
