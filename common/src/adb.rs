//! Transport classification for serial forms emitted by supported ADB listings.
//! This identifies ADB routing, not the tablet's physical radio or charging state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Transport {
    Usb,
    Network,
}

impl Transport {
    pub fn label(self) -> &'static str {
        match self {
            Self::Usb => "USB",
            Self::Network => "Network ADB",
        }
    }
}

/// ADB socket endpoints include a colon (including IPv6). Its discovery path also
/// registers mDNS instance/service names without a port. Plain device serials
/// retain the existing USB classification; never infer physical device identity
/// from a network service's instance-name prefix.
/// Sources: AOSP adb_mdns.h, client/mdns_utils.cpp and client/transport_mdns.cpp.
pub fn transport_of(serial: &str) -> Transport {
    if serial.contains(':') || is_mdns_serial(serial) {
        Transport::Network
    } else {
        Transport::Usb
    }
}

fn is_mdns_serial(serial: &str) -> bool {
    let serial = serial.to_ascii_lowercase();
    let serial = serial.strip_suffix('.').unwrap_or(&serial);
    let serial = serial.strip_suffix(".local").unwrap_or(serial);
    [
        "._adb._tcp",
        "._adb-tls-connect._tcp",
        "._adb-tls-pairing._tcp",
    ]
    .iter()
    .any(|service| serial.ends_with(service))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t278_mdns_service_and_domain_forms_are_network_routes() {
        for service in ["_adb", "_adb-tls-connect", "_adb-tls-pairing"] {
            for suffix in ["", ".", ".local", ".local."] {
                let serial = format!("instance.{service}._tcp{suffix}");
                assert_eq!(transport_of(&serial), Transport::Network, "T278: {serial}");
                assert_eq!(
                    transport_of(&serial.to_ascii_uppercase()),
                    Transport::Network
                );
            }
        }
        for plain in [
            "8002RH1010011900",
            "UNKNOWN_A",
            "adb-USB_SERIAL",
            "prefix_adb-tls-connect",
        ] {
            assert_eq!(transport_of(plain), Transport::Usb);
        }
    }
}

/// Decode a bounded physical identity from an ADB property response.
/// The returned view borrows only this response; callers own any retained copy.
pub fn physical_identity(bytes: &[u8]) -> Option<&str> {
    let identity = std::str::from_utf8(bytes).ok()?.trim();
    crate::tablets::validate_adb_serial(identity).ok()?;
    (!identity.eq_ignore_ascii_case("unknown")).then_some(identity)
}

#[cfg(test)]
mod identity_tests {
    #[test]
    fn t1383_physical_identity_contract_boundaries() {
        for bytes in [
            vec![0xff],
            vec![0xfe],
            vec![],
            b"unknown".to_vec(),
            b" UnKnOwN\n".to_vec(),
            vec![b'a'; 1025],
        ] {
            assert_eq!(super::physical_identity(&bytes), None);
        }
        for code in (0..=31).chain(127..=159) {
            let value = format!("a{}b", char::from_u32(code).unwrap());
            assert_eq!(super::physical_identity(value.as_bytes()), None);
        }
        for identity in ["café".to_owned(), "x".repeat(1024)] {
            let response = format!("{identity}\r\n");
            assert_eq!(
                super::physical_identity(response.as_bytes()),
                Some(identity.as_str())
            );
        }
    }
}
