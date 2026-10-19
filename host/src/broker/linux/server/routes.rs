use super::*;
use blent_config::connection::registration_client::Request as RegistrationRequest;
use std::os::fd::OwnedFd;
impl Server {
    pub(super) fn register(
        &mut self,
        client: u64,
        request: u64,
        registration: RegistrationRequest,
        descriptors: Vec<OwnedFd>,
    ) -> Result<()> {
        ensure!(
            self.routes.len() < self.settings.registrations,
            "registration capacity reached"
        );
        ensure!(
            self.owner_id < i64::MAX as u64,
            "registration owner IDs exhausted"
        );
        let RegistrationRequest {
            peer,
            resource,
            credential,
            profile,
        } = registration;
        let connection = self.connection_for(&peer)?;
        let mut endpoints = Endpoints::validate(resource, descriptors)?;
        self.owner_id += 1;
        let owner = self.owner_id;
        let route = endpoints.reserve(&mut self.claims, owner)?;
        let offer = Offer {
            resource,
            owner,
            credential,
            profile,
            connect: endpoints.connect()?,
        };
        if let Err(error) = self.send_link(connection, link::Command::Register(offer)) {
            self.claims.revoke(route)?;
            endpoints.retire(&mut self.claims, route)?;
            return Err(error);
        }
        self.routes.insert(
            owner,
            Registration {
                client,
                request,
                connection,
                route,
                endpoints,
                lease: None,
                closing: false,
            },
        );
        self.reply(
            client,
            vec![
                Message::Registered { request, route },
                Message::Status {
                    request,
                    route,
                    state: RouteState::Queued,
                },
            ],
        );
        Ok(())
    }
    fn connection_for(&self, peer: &PeerId) -> Result<u64> {
        let mut matching = self.links.keys().copied().filter(|&id| {
            self.controller
                .lease(id)
                .is_some_and(|lease| &lease.ticket.peer == peer)
        });
        let connection = matching.next().context("peer not authenticated")?;
        ensure!(matching.next().is_none(), "ambiguous authenticated peer");
        Ok(connection)
    }
    pub(super) fn unregister(&mut self, client: u64, request: u64, route: RouteId) -> Result<()> {
        let registered = self.routes.get_mut(&route.owner).context("unknown route")?;
        ensure!(
            registered.client == client && registered.route == route,
            "foreign or stale route"
        );
        registered.request = request;
        self.cancel_route(route.owner)
    }
    pub(super) fn cancel_route(&mut self, owner: u64) -> Result<()> {
        let route = self.routes.get_mut(&owner).context("unknown route")?;
        self.claims.revoke(route.route)?;
        route.closing = true;
        let id = route.connection;
        let lease = route.lease.clone();
        if let Some(lease) = lease {
            if self.send_link(id, link::Command::Cancel(lease)).is_err() {
                self.controller.failed(id)?;
            }
        }
        self.status(owner, RouteState::Closing);
        Ok(())
    }
    pub(super) fn registered(&mut self, id: u64, owner: u64, result: Result<Lease>) -> Result<()> {
        let route = self
            .routes
            .get_mut(&owner)
            .context("unknown registration reply")?;
        ensure!(
            route.connection == id && route.lease.is_none(),
            "stale registration reply"
        );
        match result {
            Ok(lease) => {
                ensure!(lease.owner == owner, "foreign resource owner");
                route.lease = Some(lease);
                if route.closing {
                    self.cancel_route(owner)?;
                }
            }
            Err(_) => {
                self.status(owner, RouteState::Failed);
                self.retire_route(owner)?;
            }
        }
        Ok(())
    }
    pub(super) fn route_state(
        &mut self,
        id: u64,
        lease: Lease,
        state: session::State,
    ) -> Result<()> {
        let route = self
            .routes
            .get(&lease.owner)
            .context("unknown resource status")?;
        ensure!(
            route.connection == id && route.lease.as_ref() == Some(&lease),
            "stale resource status"
        );
        match state {
            session::State::PermissionRequired => (),
            session::State::Admitted => {
                if !route.closing {
                    self.status(lease.owner, RouteState::Active);
                }
            }
            session::State::Closing => self.cancel_route(lease.owner)?,
            session::State::Retired => self.retire_route(lease.owner)?,
        }
        Ok(())
    }
    fn status(&mut self, owner: u64, state: RouteState) {
        if let Some(route) = self.routes.get(&owner) {
            self.reply(
                route.client,
                vec![Message::Status {
                    request: route.request,
                    route: route.route,
                    state,
                }],
            );
        }
    }
    fn retire_route(&mut self, owner: u64) -> Result<()> {
        let route = self
            .routes
            .get_mut(&owner)
            .context("unknown retiring route")?;
        self.claims.revoke(route.route)?;
        route.endpoints.retire(&mut self.claims, route.route)?;
        let retired = self.routes.remove(&owner).unwrap();
        self.reply(
            retired.client,
            vec![Message::Status {
                request: retired.request,
                route: retired.route,
                state: RouteState::Retired,
            }],
        );
        Ok(())
    }
    pub(super) fn retire_link_routes(&mut self, id: u64) -> Result<()> {
        let owners: Vec<_> = self
            .routes
            .iter()
            .filter_map(|(&owner, route)| (route.connection == id).then_some(owner))
            .collect();
        for owner in owners {
            self.retire_route(owner)?;
        }
        Ok(())
    }
}
