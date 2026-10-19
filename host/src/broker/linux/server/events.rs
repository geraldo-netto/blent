use super::*;
impl Server {
    pub(super) fn link_event(&mut self, id: u64, event: link::Event) -> Result<()> {
        match event {
            link::Event::Identified(peer) => {
                let action = self
                    .controller
                    .identified(id, peer, link::now(self.origin)?)?;
                self.action(action)
            }
            link::Event::Verified(verified, limits) => {
                let link = self.links.get_mut(&id).context("unknown link")?;
                ensure!(link.verified.is_none(), "duplicate native TLS proof");
                link.verified = Some((verified, limits));
                Ok(())
            }
            link::Event::Registered { owner, result } => self.registered(id, owner, result),
            link::Event::State(lease, state) => self.route_state(id, lease, state),
        }
    }
    pub(super) fn authorize_next(&mut self) -> Result<()> {
        if self.authority_busy {
            return Ok(());
        }
        if let Some(job) = self.controller.next_authority()? {
            self.authority
                .submit(job)
                .map_err(|_| anyhow::anyhow!("trust worker unavailable"))?;
            self.authority_busy = true;
            return Ok(());
        }
        let Some((&id, link)) = self.links.iter().find(|(_, link)| link.verified.is_some()) else {
            return Ok(());
        };
        let (verified, _) = link.verified.as_ref().unwrap();
        let job = match self.controller.prepare_authentication(
            id,
            &verified.ticket,
            verified.peer.clone(),
            link::now(self.origin)?,
        ) {
            Ok(job) => job,
            Err(_) => {
                self.controller.failed(id)?;
                return self.close_requested();
            }
        };
        self.authority
            .submit(job)
            .map_err(|_| anyhow::anyhow!("trust worker unavailable"))?;
        self.authority_busy = true;
        Ok(())
    }
    pub(super) fn authority_event(&mut self, event: AuthorityEvent) -> Result<()> {
        let AuthorityEvent::Completed(reply) = event else {
            anyhow::bail!("duplicate authority bootstrap");
        };
        self.authority_busy = false;
        let operation = reply.operation().clone();
        let result = self
            .controller
            .complete_authority(reply, link::now(self.origin)?);
        if operation.kind == AuthorityKind::Revoke {
            self.revoke_replies(&operation.peer, result.is_ok());
        }
        if let Ok(Some(admission)) = result {
            if self.admit(admission.connection, &admission.lease).is_err() {
                self.controller.failed(admission.connection)?;
            }
        }
        Ok(())
    }
    fn admit(&mut self, id: u64, lease: &blent_config::connection::pairing::Lease) -> Result<()> {
        let (verified, limits) = self
            .links
            .get_mut(&id)
            .and_then(|link| link.verified.take())
            .context("native proof already retired")?;
        let connection = Connection::new(verified, self.controller.pairing(id)?, lease, limits)?;
        self.send_link(id, link::Command::Admit(Box::new(connection)))
    }
    fn revoke_replies(&mut self, peer: &PeerId, success: bool) {
        let clients: Vec<_> = self
            .revocations
            .iter()
            .filter_map(|(&client, (request, expected))| {
                (peer == expected).then_some((client, *request))
            })
            .collect();
        for (client, request) in clients {
            self.revocations.remove(&client);
            if success {
                self.snapshot(client, request);
            } else {
                self.reject(client, request, Rejection::Failed);
            }
        }
    }
}
