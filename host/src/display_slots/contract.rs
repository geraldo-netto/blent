//! Capacity values and read-only reservation contract, independent of policy.
use blent_config::connection::pairing::PeerId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Owner {
    Adb(String),
    Peer(PeerId),
}
impl Owner {
    pub fn target(&self) -> blent_config::display_control::Target {
        use blent_config::display_control::Target;
        match self {
            Self::Adb(serial) => Target::Adb(serial.clone()),
            Self::Peer(peer) => Target::Usb(peer.as_str().to_owned()),
        }
    }
    pub fn from_target(target: &blent_config::display_control::Target) -> anyhow::Result<Self> {
        Ok(match target.identity()? {
            blent_config::tablets::Identity::Adb(serial) => Self::Adb(serial),
            blent_config::tablets::Identity::UsbPeer(peer) => Self::Peer(peer),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub(super) instance: u32,
    pub(super) revision: u64,
}
impl Ticket {
    pub fn lease(self) -> blent_config::display_control::Lease {
        blent_config::display_control::Lease {
            slot: self.instance,
            generation: self.revision,
        }
    }
    pub fn instance(self) -> u32 {
        self.instance
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    None,
    Ready(u32),
    Current(Ticket),
    Replace(Ticket),
    Wait,
}
/// Prompt reservation validation only. It is not authentication or media proof.
pub trait Reservations {
    fn open_peer(&self, ticket: Ticket, peer: &PeerId) -> bool;
}
