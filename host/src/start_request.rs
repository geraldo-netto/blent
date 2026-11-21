//! Bounded command slot carrying its own start acknowledgement.
use blent_config::media_start::{Acknowledger, Start};

#[derive(Clone)]
pub(crate) struct Request<T> {
    options: T,
    acknowledgement: Acknowledger,
}
impl<T> Request<T> {
    pub(crate) fn new(options: T) -> (Self, Start) {
        let (start, acknowledgement) = Start::pending();
        (
            Self {
                options,
                acknowledgement,
            },
            start,
        )
    }
    pub(crate) fn reject(&self) {
        self.acknowledgement.reject();
    }
    /// The supervisor has published Starting before acknowledging consumption.
    pub(crate) fn begin(self) -> T {
        self.acknowledgement.acknowledge();
        self.options
    }
}
