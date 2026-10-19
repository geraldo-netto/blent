//! Original observations have separate bounds for independently replaced owners.
use blent_config::retirement::observations::{Observations, Reservation};
use std::io;

#[derive(Clone, Copy)]
pub enum Resource {
    Connection,
    Devices,
    Producer,
    Decoder,
}
#[derive(Default)]
pub(super) struct Tracker {
    connection: Observations<1>,
    // Original blocking constructor and separately funded original-file close.
    devices: Observations<2>,
    // Two original blocking output workers and their two owned controllers.
    producers: Observations<4>,
    decoder: Observations<1>,
}
impl Tracker {
    pub(super) fn reserve(&mut self, resource: Resource) -> io::Result<Reservation> {
        match resource {
            Resource::Connection => self.connection.reserve(),
            Resource::Devices => self.devices.reserve(),
            Resource::Producer => self.producers.reserve(),
            Resource::Decoder => self.decoder.reserve(),
        }
    }
    pub(super) fn complete(&self) -> bool {
        self.connection.complete()
            && self.devices.complete()
            && self.producers.complete()
            && self.decoder.complete()
    }
}
