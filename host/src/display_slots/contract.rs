//! Capacity values and read-only reservation contract, independent of policy.
use blent_config::connection::pairing::PeerId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Owner {
    Adb(String),
    Peer(PeerId),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub(super) instance: u32,
    pub(super) revision: u64,
}
impl Ticket {
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
