//! T1544: reply identity uses the same endpoint equivalence as ownership.
use super::policy::connected;

#[test]
fn t1544_canonical_success_matches_the_original_endpoint() {
    for (saved, echoed) in [
        ("LOCALHOST:055637", "LOCALHOST:55637"),
        ("HOST:0005", "host:5"),
        ("[2001:0db8::1]:0005", "[2001:db8::1]:5"),
        ("192.0.2.1:05555", "192.0.2.1:5555"),
    ] {
        for prefix in ["connected to ", "already connected to "] {
            assert!(
                connected(format!("{prefix}{echoed}\n").as_bytes(), saved),
                "T1544: {saved} / {echoed}"
            );
        }
    }
}

#[test]
fn t1544_canonicalization_never_accepts_foreign_or_invalid_endpoints() {
    for (saved, echoed) in [
        ("HOST:0005", "host:6"),
        ("host:5", "other:5"),
        ("[2001:db8::1]:5", "[2001:db8::2]:5"),
        ("host:0", "host:0"),
        ("host:65536", "host:65536"),
        ("", ""),
        ("host:5", "host:5\nconnected to other:5"),
    ] {
        assert!(
            !connected(format!("connected to {echoed}").as_bytes(), saved),
            "T1544: {saved} / {echoed}"
        );
    }
}

#[test]
fn t1544_reply_bytes_and_port_boundaries_are_checked_before_matching() {
    for byte in 0..=255 {
        assert!(
            !connected(&[byte; 64], "host:5"),
            "T1544: reply byte {byte}"
        );
    }
    for port in [0, 1, 5, 65535, 65536, u32::MAX] {
        let saved = format!("HOST:0{port}");
        let reply = format!("connected to host:{port}");
        assert_eq!(
            connected(reply.as_bytes(), &saved),
            (1..=65535).contains(&port),
            "T1544: port {port}"
        );
    }
}
