use super::*;
use blent_config::adb_reverse::temporary::{self, Cleanup, Creation};
#[cfg(test)]
mod tests;

struct State<C> {
    adb: Adb<C>,
    control: Arc<Control>,
    initial: String,
    physical: Option<String>,
    created: [Creation; 2],
}
pub(super) async fn run<C: Commands>(
    adb: Adb<C>,
    control: Arc<Control>,
    mut requests: mpsc::Receiver<oneshot::Sender<Result<(), String>>>,
    mut stop: watch::Receiver<bool>,
    failure: watch::Sender<Option<String>>,
) {
    let Some(first) = requests.recv().await else {
        control.settled.store(true, Ordering::Release);
        return;
    };
    match State::new(adb, control.clone()).await {
        Ok(mut state) => {
            let active = state.respond(first, &stop, false).await;
            if active {
                state.serve(&mut requests, &mut stop).await;
            }
            requests.close();
            state.cleanup(&failure).await;
        }
        Err(error) => {
            let _ = first.send(Err(error.to_string()));
        }
    }
    control.settled.store(true, Ordering::Release);
}
impl<C: Commands> State<C> {
    async fn new(adb: Adb<C>, control: Arc<Control>) -> Result<Self> {
        let initial = control.serial.lock().unwrap().clone();
        let physical = match physical(&adb, &initial).await {
            Ok(identity) => Some(identity),
            Err(error) => {
                ensure!(transport_of(&initial) == Transport::Usb, "{error}");
                None
            }
        };
        let reservation = adb.0.display_route_lease(
            physical.as_deref().unwrap_or(&initial),
            (control.expected[0].1, control.expected[1].1),
        )?;
        *control.reservation.lock().unwrap() = Some(reservation);
        Ok(Self {
            adb,
            control,
            initial,
            physical,
            created: [Creation::NotStarted; 2],
        })
    }
    async fn serial(&self) -> Result<String> {
        let serial = self.control.serial.lock().unwrap().clone();
        self.check_identity(&serial).await?;
        Ok(serial)
    }
    async fn check_identity(&self, serial: &str) -> Result<()> {
        match &self.physical {
            Some(identity) => ensure!(
                physical(&self.adb, serial).await?.as_str() == identity,
                "Display route transport identity changed"
            ),
            None => ensure!(
                serial == self.initial,
                "USB serial cannot retarget without physical identity"
            ),
        }
        Ok(())
    }
    async fn cleanup_serial(&self) -> Result<String> {
        let serial = self.control.serial.lock().unwrap().clone();
        let listing = checked(&self.adb, vec!["devices".into()]).await?;
        let devices = crate::adb_inventory::entries(&listing)
            .context("Display route inventory is invalid")?;
        ensure!(
            devices.iter().any(|(device, _)| *device == serial),
            "Display route tablet absent; original cleanup retained"
        );
        self.check_identity(&serial).await?;
        Ok(serial)
    }
    async fn serve(
        &mut self,
        requests: &mut mpsc::Receiver<oneshot::Sender<Result<(), String>>>,
        stop: &mut watch::Receiver<bool>,
    ) {
        while !*stop.borrow() {
            tokio::select! {
                biased;
                _ = crate::process_retirement::PROCESS.cancelled() => break,
                _ = stop.changed() => break,
                next = requests.recv() => {
                    let Some(reply) = next else { break; };
                    if !self.respond(reply, stop, true).await { break; }
                }
            }
        }
    }
    async fn respond(
        &mut self,
        reply: oneshot::Sender<Result<(), String>>,
        stop: &watch::Receiver<bool>,
        refresh: bool,
    ) -> bool {
        let (serial, missing) = match self.missing().await {
            Ok(routes) => routes,
            // Read-only refresh failure does not revoke existing mappings. A
            // failed first admission still retires its original reservation.
            Err(error) => return reply.send(Err(error.to_string())).is_ok() && refresh,
        };
        let result = self
            .prepare(&serial, missing, &reply, stop)
            .await
            .map_err(|error| error.to_string());
        let active = result.is_ok();
        reply.send(result).is_ok() && active
    }
    async fn missing(&self) -> Result<(String, Vec<(u16, u16)>)> {
        let serial = self.serial().await?;
        let listing = listing(&self.adb, &serial).await?;
        let missing = blent_config::adb_reverse::missing(&listing, &self.control.expected)?;
        Ok((serial, missing))
    }
    async fn prepare(
        &mut self,
        serial: &str,
        missing: Vec<(u16, u16)>,
        reply: &oneshot::Sender<Result<(), String>>,
        stop: &watch::Receiver<bool>,
    ) -> Result<()> {
        for route in missing {
            ensure!(
                !*stop.borrow() && !reply.is_closed(),
                "Display route admission cancelled"
            );
            let index = self
                .control
                .expected
                .iter()
                .position(|expected| *expected == route)
                .unwrap();
            self.create(index, serial).await?;
        }
        ensure!(
            !*stop.borrow() && !reply.is_closed(),
            "Display route admission cancelled"
        );
        Ok(())
    }
    async fn create(&mut self, index: usize, serial: &str) -> Result<()> {
        let (remote, local) = self.control.expected[index];
        self.created[index] = Creation::Uncertain;
        let attempt = self
            .adb
            .0
            .execute_route(args(
                serial,
                &[
                    "reverse",
                    "--no-rebind",
                    &format!("tcp:{remote}"),
                    &format!("tcp:{local}"),
                ],
            ))
            .await;
        if !attempt.started {
            self.created[index] = Creation::NotStarted;
        }
        ensure!(
            attempt.output?.status.success(),
            "Display mapping creation was not acknowledged"
        );
        self.created[index] = Creation::Acknowledged;
        Ok(())
    }
    async fn cleanup(&mut self, failure: &watch::Sender<Option<String>>) {
        let mut retry = crate::cleanup_retry::Retry::new();
        loop {
            match self.cleanup_once().await {
                Ok(true) => return,
                Ok(false) => {
                    if self.created.iter().any(|state| {
                        matches!(state, Creation::Uncertain | Creation::RemovalPending)
                    }) {
                        failure
                            .send_replace(Some("Display mapping outcome remains unsettled".into()));
                    }
                }
                Err(error) => {
                    failure.send_replace(Some(error.to_string()));
                }
            }
            tokio::time::sleep(retry.next()).await;
        }
    }
    async fn cleanup_once(&mut self) -> Result<bool> {
        if self
            .created
            .iter()
            .all(|state| *state == Creation::NotStarted)
        {
            return Ok(true);
        }
        let serial = self.cleanup_serial().await?;
        let listing = listing(&self.adb, &serial).await?;
        for index in 0..2 {
            self.cleanup_route(index, &serial, &listing).await?;
        }
        Ok(self
            .created
            .iter()
            .all(|state| *state == Creation::NotStarted))
    }
    async fn cleanup_route(&mut self, index: usize, serial: &str, listing: &str) -> Result<()> {
        let (remote, local) = self.control.expected[index];
        let remote = format!("tcp:{remote}");
        let route = temporary::route(listing, &remote, &format!("tcp:{local}"))?;
        match self.created[index].cleanup(route) {
            Cleanup::Complete => self.created[index] = Creation::NotStarted,
            Cleanup::Wait => {}
            Cleanup::Remove => {
                self.created[index] = Creation::RemovalPending;
                let attempt = self
                    .adb
                    .0
                    .execute_route(args(serial, &["reverse", "--remove", &remote]))
                    .await;
                if !attempt.started {
                    self.created[index] = Creation::Acknowledged;
                }
                ensure!(
                    attempt.output?.status.success(),
                    "Display mapping removal was not acknowledged"
                );
                self.created[index] = Creation::Acknowledged;
            }
        }
        Ok(())
    }
}
async fn checked<C: Commands>(adb: &Adb<C>, arguments: Vec<String>) -> Result<String> {
    let output = adb.0.execute_route(arguments).await.output?;
    ensure!(output.status.success(), "Display route query failed");
    ensure!(
        output.stdout.len() <= 65536,
        "Display route query exceeds 64 KiB"
    );
    String::from_utf8(output.stdout).context("Display route query is not UTF-8")
}
async fn listing<C: Commands>(adb: &Adb<C>, serial: &str) -> Result<String> {
    checked(adb, args(serial, &["reverse", "--list"])).await
}
async fn physical<C: Commands>(adb: &Adb<C>, serial: &str) -> Result<String> {
    let value = checked(adb, args(serial, &["shell", "getprop", "ro.serialno"])).await?;
    blent_config::adb::physical_identity(value.as_bytes())
        .map(str::to_owned)
        .context("Display route physical identity unavailable")
}
