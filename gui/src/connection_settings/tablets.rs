//! Bounded descriptive projection of one immutable, validated broker snapshot.
//! Connection IDs join names; only the authenticated peer supplies identity.
use blent_config::{
    connection::{
        control::{DeviceNames, State},
        control_view::{Link, Snapshot},
    },
    tablets::Identity,
    workspace::{Device, Presence},
};

pub(super) fn project(snapshot: &Snapshot, connection: u64, link: &Link) -> Option<Device> {
    if link.state != State::Authenticated {
        return None;
    }
    let identity = Identity::UsbPeer(link.ticket.as_ref()?.peer.clone());
    let name = snapshot
        .devices()
        .values()
        .find(|device| device.connection == Some(connection))
        .map_or_else(String::new, |device| label(&device.names));
    Device::new(identity, name, Presence::Connected, false).ok()
}

// Each validated descriptor is at most 256 bytes; the result is at most 515.
fn label(names: &DeviceNames) -> String {
    match (names.model(), names.manufacturer()) {
        ("", "") => String::new(),
        (model, "") => model.to_owned(),
        ("", maker) => format!("USB tablet ({maker})"),
        (model, maker) => format!("{model} ({maker})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1375_missing_descriptors_have_explicit_bounded_fallbacks() {
        for (model, maker, expected) in [
            ("", "", ""),
            ("Tablet", "", "Tablet"),
            ("", "Maker", "USB tablet (Maker)"),
            ("Tablet", "Maker", "Tablet (Maker)"),
        ] {
            let names = DeviceNames::new(maker.into(), model.into()).unwrap();
            assert_eq!(label(&names), expected);
        }
        let names = DeviceNames::new("m".repeat(256), "x".repeat(256)).unwrap();
        assert_eq!(label(&names).len(), 515);
    }
}
