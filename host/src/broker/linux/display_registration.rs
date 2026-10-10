//! Native listener capabilities bound to the portable display-registration port.
use super::registration_client::Factory;
use crate::{registered_display::Registration, session_network::RetainedListeners};
use anyhow::Result;
use blent_config::connection::registration_client::{Client, Request};
use std::sync::Arc;

pub struct Bound {
    factory: Arc<Factory>,
    listeners: Arc<RetainedListeners>,
}
impl Bound {
    pub fn new(factory: Arc<Factory>, listeners: Arc<RetainedListeners>) -> Self {
        Self { factory, listeners }
    }
}
impl Registration for Bound {
    fn start(&mut self, request: Request) -> Result<Box<dyn Client>> {
        self.factory
            .start(request, self.listeners.try_clone()?)
            .map(|client| Box::new(client) as Box<dyn Client>)
    }
}
