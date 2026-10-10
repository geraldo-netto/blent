//! Existing ADB discovery boundary. Its identity prefixes/classification never
//! enter the portable attachment owner or authenticated native connection API.
use super::{Attachment, Identity, Link};
use blent_config::adb::Transport;

impl Attachment {
    pub fn begin(&self, identity: Option<String>) {
        self.begin_with_transport(identity, None);
    }
    pub fn begin_with_transport(&self, identity: Option<String>, transport: Option<Transport>) {
        let identity = identity.and_then(observed);
        let link = transport.map(|route| match route {
            Transport::Usb => Link::Usb,
            Transport::Network => Link::Network,
        });
        // A malformed observation invalidates old authority, just like detach.
        self.begin_route(identity, link);
    }
}
fn observed(value: String) -> Option<Identity> {
    let profile = value
        .strip_prefix("device:")
        .filter(|id| !id.is_empty())
        .map(str::to_owned);
    Identity::observed(value, profile).ok()
}
