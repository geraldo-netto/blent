use super::*;
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
                peer,
                resource,
                credential,
                profile,
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
        if self.inventory_error && matches!(command, Command::Inspect | Command::Connect { .. }) {
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
        let settings = link::Settings {
            identity: self.identity.identity.clone(),
            limits: self.settings.session,
            block: self.settings.block,
            setup_timeout: Duration::from_millis(self.settings.control.setup_ms as u64),
            approval_timeout: Duration::from_millis(self.settings.control.approval_ms as u64),
            origin: self.origin,
        };
        let handle = match Handle::start_factory(
            &self.link_pool,
            move || accessory::open_selected(&helper, &target),
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
        let candidates = match result {
            Ok(candidates) => candidates,
            Err(error) => {
                self.inventory_error = true;
                tracing::warn!("USB inventory unavailable: {error:#}");
                return self.controller.replace_devices(Vec::new());
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
                    payload,
                }
            })
            .collect();
        self.controller.replace_devices(devices)?;
        self.inventory_error = false;
        Ok(())
    }
}
