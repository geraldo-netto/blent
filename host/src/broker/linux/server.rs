//! Linux composition root. Domain authority and wire contracts remain portable.
use super::{
    async_io::AsyncListener, inventory::Inventory, owner::Owner, peer::Peer, state, Endpoints,
    Received,
};
use crate::{
    accessory::{
        linux::{
            self as accessory,
            discovery::{Candidate, Support},
        },
        setup::Verified,
    },
    broker::{
        authority_worker::{Event as AuthorityEvent, Ready, Worker},
        bridges::Offer,
        connection::Connection,
        control::{Action, AuthorityKind, Controller, Device},
        link::{self, Handle},
        session,
    },
};
use anyhow::{ensure, Context, Result};
use blent_config::{
    connection::{
        control::Command,
        pairing::PeerId,
        registration::{Message, Rejection, State as RouteState},
        resources::Lease,
        routes::{RouteClaims, RouteId},
    },
    retirement::{Completion, Pool},
};
use std::{collections::BTreeMap, future::Future, pin::Pin, time::Duration};
use tokio::time::Instant;
pub(super) mod authority;
mod commands;
mod events;
mod routes;
mod settings;
mod shutdown;
pub use settings::Settings;
type Control = Controller<(u8, Vec<u8>), Candidate>;
struct Client {
    peer: Option<Peer>,
    completion: Completion,
}
struct Link {
    handle: Option<Handle>,
    completion: Completion,
    verified: Option<(Verified, session::Limits)>,
}
struct Registration {
    client: u64,
    request: u64,
    connection: u64,
    route: RouteId,
    endpoints: Endpoints,
    lease: Option<Lease>,
    closing: bool,
}
enum Input {
    Client(u64, Result<Received>),
    Link(u64, Result<link::Event>),
}
pub struct Server {
    settings: Settings,
    listener: AsyncListener,
    authority: Worker,
    identity: Ready,
    controller: Control,
    clients: BTreeMap<u64, Client>,
    links: BTreeMap<u64, Link>,
    routes: BTreeMap<u64, Registration>,
    claims: RouteClaims,
    link_pool: Pool,
    inventory: Option<Inventory>,
    inventory_error: bool,
    next_inventory: Instant,
    origin: Instant,
    owner_id: u64,
    authority_busy: bool,
    revocations: BTreeMap<u64, (u64, PeerId)>,
}
impl Server {
    pub async fn open(settings: Settings) -> Result<Self> {
        Self::open_in(settings, None).await
    }
    /// Explicit isolated contexts belong to independent backend fixtures. Normal
    /// process startup always uses the shared authority admission domain.
    pub(super) async fn open_in(settings: Settings, context: Option<&Pool>) -> Result<Self> {
        settings.validate()?;
        let pool = match context {
            Some(pool) => pool,
            None => authority::shared()?,
        };
        let admission = pool.reserve()?;
        settings.directories()?;
        let listener = AsyncListener::new(Owner::bind(
            &settings.runtime,
            settings.control.clients as u32,
        )?)?;
        let state = settings.state.clone();
        let peers = settings.trust_peers;
        let mut authority = Worker::start_reserved(admission, move || state::open(&state, peers))?;
        let AuthorityEvent::Ready(ready) = authority.next().await? else {
            anyhow::bail!("unexpected authority bootstrap");
        };
        let identity = ready?;
        let controller = Controller::new(
            identity.local.clone(),
            identity.saved.clone(),
            settings.control,
        )?;
        let claims = RouteClaims::new(settings.registrations)?;
        let link_pool = Pool::new(settings.control.links)?;
        Ok(Self {
            settings,
            listener,
            authority,
            identity,
            controller,
            claims,
            link_pool,
            clients: BTreeMap::new(),
            links: BTreeMap::new(),
            routes: BTreeMap::new(),
            inventory: None,
            inventory_error: true,
            next_inventory: Instant::now(),
            origin: Instant::now(),
            owner_id: 0,
            authority_busy: false,
            revocations: BTreeMap::new(),
        })
    }
    pub async fn run(&mut self) -> Result<()> {
        let mut clock = tokio::time::interval(Duration::from_millis(20));
        clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                accepted = self.listener.accept() => self.accept(accepted?.0)?,
                input = next(&mut self.clients, &mut self.links) => self.input(input)?,
                reply = self.authority.next() => self.authority_event(reply?)?,
                inventory = inventory_next(&mut self.inventory) => self.inventory_result(inventory)?,
                _ = clock.tick() => self.tick()?,
            }
            self.close_requested()?;
            self.authorize_next()?;
        }
    }
    fn accept(&mut self, socket: super::async_io::AsyncSocket) -> Result<()> {
        let Ok(id) = self.controller.admit_client() else {
            return Ok(());
        };
        match Peer::start(socket, self.settings.session.io_timeout) {
            Ok(peer) => {
                self.clients.insert(
                    id,
                    Client {
                        completion: peer.completion(),
                        peer: Some(peer),
                    },
                );
            }
            Err(_) => {
                self.controller.drop_client(id);
            }
        }
        Ok(())
    }
    fn input(&mut self, input: Input) -> Result<()> {
        match input {
            Input::Client(id, Ok(received)) => self.request(id, received),
            Input::Client(id, Err(_)) => self.close_client(id),
            Input::Link(id, Ok(event)) => {
                if self.link_event(id, event).is_err() {
                    self.controller.failed(id)?;
                }
                Ok(())
            }
            Input::Link(id, Err(_)) => self.controller.failed(id),
        }
    }
    fn reply(&mut self, client: u64, messages: Vec<Message>) {
        let failed = self
            .clients
            .get(&client)
            .and_then(|client| client.peer.as_ref())
            .is_some_and(|peer| peer.reply(messages).is_err());
        if failed {
            let _ = self.close_client(client);
        }
    }
    fn reject(&mut self, client: u64, request: u64, reason: Rejection) {
        self.reply(client, vec![Message::Rejected { request, reason }]);
    }
    fn snapshot(&mut self, client: u64, request: u64) {
        self.reply(
            client,
            self.controller
                .snapshot()
                .into_iter()
                .map(|notice| Message::Report { request, notice })
                .collect(),
        );
    }
    fn close_client(&mut self, client: u64) -> Result<()> {
        if let Some(peer) = self
            .clients
            .get_mut(&client)
            .and_then(|client| client.peer.take())
        {
            peer.stop();
        }
        self.revocations.remove(&client);
        let owners: Vec<_> = self
            .routes
            .iter()
            .filter_map(|(&id, route)| (route.client == client).then_some(id))
            .collect();
        for owner in owners {
            self.cancel_route(owner)?;
        }
        Ok(())
    }
    fn close_requested(&mut self) -> Result<()> {
        for id in self.controller.closing() {
            if let Some(link) = self.links.get_mut(&id) {
                if let Some(handle) = link.handle.take() {
                    handle.stop();
                }
                link.verified.take();
            }
        }
        Ok(())
    }
    fn tick(&mut self) -> Result<()> {
        self.controller.tick(link::now(self.origin)?)?;
        self.collect_clients();
        self.collect_links()?;
        self.scan()?;
        Ok(())
    }
    fn collect_clients(&mut self) {
        self.clients.retain(|&id, client| {
            let done = client.peer.is_none() && client.completion.is_complete();
            if done {
                self.controller.drop_client(id);
            }
            !done
        });
    }
    fn collect_links(&mut self) -> Result<()> {
        let done: Vec<_> = self
            .links
            .iter()
            .filter_map(|(&id, link)| {
                (link.handle.is_none() && link.completion.is_complete()).then_some(id)
            })
            .collect();
        for id in done {
            self.retire_link_routes(id)?;
            self.controller.native_finished(id)?;
            // Keep controller's native Completion tombstone, but drop the task
            // record so native_finished is never delivered twice.
            self.links.remove(&id);
        }
        self.controller.collect_retired()?;
        Ok(())
    }
}
type Pending<'a> = Pin<Box<dyn Future<Output = Input> + Send + 'a>>;
async fn next(clients: &mut BTreeMap<u64, Client>, links: &mut BTreeMap<u64, Link>) -> Input {
    let mut pending: Vec<Pending<'_>> = Vec::new();
    for (&id, client) in clients {
        if let Some(peer) = &mut client.peer {
            pending.push(Box::pin(
                async move { Input::Client(id, peer.next().await) },
            ));
        }
    }
    for (&id, link) in links {
        if let Some(handle) = &mut link.handle {
            pending.push(Box::pin(
                async move { Input::Link(id, handle.next().await) },
            ));
        }
    }
    if pending.is_empty() {
        return std::future::pending().await;
    }
    futures_util::future::select_all(pending).await.0
}
async fn inventory_next(inventory: &mut Option<Inventory>) -> Result<Vec<Candidate>> {
    match inventory {
        Some(inventory) if !inventory.finished() => inventory.next().await,
        _ => std::future::pending().await,
    }
}

#[cfg(test)]
mod tests;
