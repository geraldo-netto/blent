use super::*;
use blent_config::connection::registration_client::Request as RegistrationRequest;
impl Server {
    pub(super) fn request(&mut self, client: u64, received: Received) -> Result<()> {
        let request = received.message.request();
        let result = match received.message {
            Message::Control { command, .. } => self.control_command(client, request, command),
            Message::Register {
                peer,
                resource,
                credential,
                profile,
                ..
            } => self.register(
                client,
                request,
                RegistrationRequest {
                    peer,
                    resource,
                    credential,
                    profile,
                },
                received.descriptors,
            ),
            Message::Unregister { route, .. } => self.unregister(client, request, route),
            _ => Err(anyhow::anyhow!("client sent a reply")),
        };
        if result.is_err() {
            self.reject(client, request, Rejection::Failed);
        }
        Ok(())
    }
    fn control_command(&mut self, client: u64, request: u64, command: Command) -> Result<()> {
        if command == Command::Inspect {
            return self.inspect(client, request);
        }
        if !self.controller.inventory_available() && matches!(command, Command::Connect { .. }) {
            self.reject(client, request, Rejection::Unavailable);
            return Ok(());
        }
        let revoke = if let Command::Revoke { peer } = &command {
            ensure!(
                !self.revocations.contains_key(&client),
                "client revoke already pending"
            );
            Some(peer.clone())
        } else {
            None
        };
        let action = self
            .controller
            .command(client, command, link::now(self.origin)?)?;
        self.action(action)?;
        if let Some(peer) = revoke {
            self.revocations.insert(client, (request, peer));
        } else {
            self.snapshot(client, request);
        }
        Ok(())
    }
    pub(super) fn action(&mut self, action: Action<Candidate>) -> Result<()> {
        match action {
            Action::None | Action::Snapshot(_) | Action::Retire(_) => Ok(()),
            Action::Authenticate { connection, ticket } => {
                self.send_link(connection, link::Command::Approve(ticket))
            }
            Action::Start { connection, target } => self.start(connection, target),
        }
    }
    fn start(&mut self, connection: u64, target: Candidate) -> Result<()> {
        let helper = self.settings.helper.clone();
        let runtime = self.settings.runtime.clone();
        let settings = self
            .settings
            .link(self.identity.identity.clone(), self.origin)?;
        let handle = match Handle::start_factory(
            &self.link_pool,
            move || accessory::open_selected(&helper, &runtime, &target),
            settings,
        ) {
            Ok(handle) => handle,
            Err(error) => {
                self.controller.start_failed(connection)?;
                return Err(error);
            }
        };
        self.controller
            .attach(connection, handle.native_completion())?;
        self.links.insert(
            connection,
            Link {
                completion: handle.task_completion(),
                handle: Some(handle),
                verified: None,
            },
        );
        Ok(())
    }
    pub(super) fn send_link(&mut self, id: u64, command: link::Command) -> Result<()> {
        let handle = self
            .links
            .get(&id)
            .and_then(|link| link.handle.as_ref())
            .context("link closing")?;
        if handle.submit(command).is_err() {
            self.controller.failed(id)?;
            anyhow::bail!("link command capacity exhausted");
        }
        Ok(())
    }
    pub(super) fn scan(&mut self) -> Result<()> {
        if self.inventory.as_ref().is_some_and(Inventory::retired) {
            self.inventory = None;
        }
        if !self.controller.inventory_needed() {
            // Keep any admitted helper until its original native retirement.
            // A later client must await fresh metadata, not this idle cache.
            self.inventory_observed = false;
            self.next_inventory = Instant::now();
            return self.controller.inventory_unavailable();
        }
        if self.inventory.is_none() && Instant::now() >= self.next_inventory {
            self.next_inventory = Instant::now() + Duration::from_secs(2);
            let limits = accessory::discovery::Limits {
                devices: self.settings.control.devices,
                timeout: Duration::from_secs(5),
            };
            self.inventory = Some(Inventory::start(self.settings.helper.clone(), limits)?);
        }
        Ok(())
    }
    pub(super) fn inventory_result(&mut self, result: Result<Vec<Candidate>>) -> Result<()> {
        self.update_inventory(result)?;
        self.inventory_observed = true;
        self.complete_inspections();
        Ok(())
    }

    fn update_inventory(&mut self, result: Result<Vec<Candidate>>) -> Result<()> {
        let candidates = match result {
            Ok(candidates) => candidates,
            Err(error) => {
                tracing::warn!("USB inventory unavailable: {error:#}");
                return self.controller.inventory_unavailable();
            }
        };
        let devices = candidates
            .into_iter()
            .map(|payload| {
                let (bus, ports) = payload.physical_path();
                Device {
                    key: (bus, ports.to_vec()),
                    vendor: payload.vendor,
                    product: payload.product,
                    accessory: payload.support == Support::Accessory,
                    names: payload.names.clone(),
                    availability: payload.availability,
                    payload,
                }
            })
            .collect();
        self.controller.replace_devices(devices)
    }

    pub(super) fn automatic(&mut self) -> Result<()> {
        if !self.controller.inventory_available() {
            return Ok(());
        }
        let action = self.controller.automatic(link::now(self.origin)?)?;
        if let Err(error) = self.action(action) {
            tracing::warn!("Automatic USB connection unavailable: {error:#}");
        }
        Ok(())
    }
}
