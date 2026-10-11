//! Linux FD/process boundary for the portable original media connection owner.
use super::Bridge;
use crate::{
    broker::linux::{client::Launch, presentation, registration_client},
    connection_discovery::Discovery,
    media_registration::MediaRegistration,
};
use anyhow::Result;
use blent_config::connection::registration_client::{Client, Factory, Request};
use std::{
    net::TcpListener,
    sync::Arc,
    time::{Duration, Instant},
};

struct Bound {
    factory: Arc<registration_client::Factory>,
    listener: TcpListener,
}
impl Factory for Bound {
    fn start(&mut self, request: Request) -> Result<Box<dyn Client>> {
        self.factory
            .start(request, vec![self.listener.try_clone()?])
            .map(|client| Box::new(client) as Box<dyn Client>)
    }
}

pub(crate) fn native<P: Clone + PartialEq + Send + Sync>(
    profile: &P,
    token: &str,
    request: Request,
    listener: TcpListener,
    launch: Launch,
) -> Result<Bridge<P>> {
    crate::media_request::validate_original(&request, token)?;
    let source = Discovery::new(
        Box::new(presentation::Factory::new(presentation::Settings::new(
            launch.clone(),
        ))?),
        Duration::from_secs(1),
        Instant::now(),
    )?;
    let bound = Bound {
        factory: Arc::new(registration_client::Factory::new(
            registration_client::Settings::new(launch),
        )?),
        listener,
    };
    let owner = MediaRegistration::new(request, source, Box::new(bound))?;
    Bridge::owned(profile, token, owner)
}
