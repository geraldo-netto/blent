//! Completed ADB server replies that prove transport creation never began.
//! A generic failed-connect reply can follow registration and is not proof.

pub(super) fn not_created(bytes: &[u8], address: &str) -> bool {
    let Some(target) = error_target(address) else {
        return false;
    };
    let Some(text) = reply_line(bytes) else {
        return false;
    };
    let (host, port) = target.rsplit_once(':').expect("validated host and port");
    let prefixes = [
        format!("failed to connect to '{target}': "),
        format!("cannot connect to {target}: "),
        format!("failed to resolve host: '{host}': "),
        format!("cannot resolve host '{host}' and port {port}: "),
    ];
    let reason = prefixes.iter().find_map(|prefix| text.strip_prefix(prefix));
    reason.is_some_and(|reason| !reason.is_empty() && !reason.chars().any(char::is_control))
}

fn reply_line(bytes: &[u8]) -> Option<&str> {
    if bytes.len() > 65_536 {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    Some(
        text.strip_suffix("\r\n")
            .or_else(|| text.strip_suffix('\n'))
            .unwrap_or(text),
    )
}

fn error_target(address: &str) -> Option<String> {
    if !super::valid_address(address) {
        return None;
    }
    let (host, port) = address.rsplit_once(':')?;
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    Some(format!("{host}:{}", port.parse::<u16>().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1538_only_exact_bounded_precreation_refusals_release_the_endpoint() {
        for (address, target) in [
            ("host:5", "host:5"),
            ("HOST:0005", "HOST:5"),
            ("192.0.2.3:5555", "192.0.2.3:5555"),
            ("[2001:0db8::1]:5555", "2001:0db8::1:5555"),
        ] {
            let (host, port) = target.rsplit_once(':').unwrap();
            for prefix in [
                format!("failed to connect to '{target}': "),
                format!("cannot connect to {target}: "),
                format!("failed to resolve host: '{host}': "),
                format!("cannot resolve host '{host}' and port {port}: "),
            ] {
                for ending in ["", "\n", "\r\n"] {
                    assert!(not_created(
                        format!("{prefix}Connection refused{ending}").as_bytes(),
                        address
                    ));
                }
                assert!(!not_created(prefix.as_bytes(), address));
            }
        }
    }

    #[test]
    fn t1538_ambiguous_or_foreign_replies_and_invalid_addresses_stay_owned() {
        for text in [
            "failed to connect to host:5",
            "failed to authenticate to host:5",
            "connected to host:5",
            "failed to connect to 'other:5': Connection refused",
            "failed to connect to 'host:5': refused\nconnected to host:5",
            "failed to connect to 'host:5': refused\n\n",
            "failed to connect to 'host:5': \0",
        ] {
            assert!(!not_created(text.as_bytes(), "host:5"), "{text:?}");
        }
        for address in ["", "host:0", "host:65536", "host:5\n", "0.0.0.0:5"] {
            assert!(!not_created(
                b"failed to connect to 'host:5': refused",
                address
            ));
        }
        for byte in 0..=255 {
            assert!(!not_created(&[byte; 64], "host:5"));
        }
        let prefix = "failed to connect to 'host:5': ";
        let limit = format!("{prefix}{}", "x".repeat(65_536 - prefix.len()));
        assert!(not_created(limit.as_bytes(), "host:5"));
        assert!(!not_created(format!("{limit}x").as_bytes(), "host:5"));
    }
}
