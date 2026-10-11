//! Linux daemon composition. Portable connection policy owns no native handles.
use super::*;
use blent::{
    broker::linux::{
        client::Launch, display_registration::Bound, presentation, registration_client,
    },
    connection_discovery::Discovery,
    display_connections::{Admission, Connections, Decision},
    registered_session::Bindings,
    session_network::RetainedListeners,
    session_work::RuntimeWork,
};

type Started = (Admission, Result<Bindings>);
struct Primary {
    attachment: attachment::Attachment,
    status: watch::Receiver<blent::session_state::State>,
    listeners: Arc<RetainedListeners>,
}
pub(crate) struct Native {
    connections: Connections,
    registration: Arc<registration_client::Factory>,
    primary: Primary,
    startups: JoinSet<Started>,
}
impl Native {
    pub(super) fn new(
        config: &Config,
        runtime: &ExtraSession,
        listeners: RetainedListeners,
        launch: Launch,
    ) -> Result<Self> {
        anyhow::ensure!(
            runtime.instance == 0
                && listeners.ports() == config.ports
                && listeners.ports() == (runtime.video_port, runtime.input_port),
            "invalid primary USB listener handoff"
        );
        let ports = config::slot_ports(config.ports.0, config.ports.1, config.extra.max_tablets)?;
        let source = Discovery::new(
            Box::new(presentation::Factory::new(presentation::Settings::new(
                launch.clone(),
            ))?),
            Duration::from_secs(5),
            Instant::now(),
        )?;
        Ok(Self {
            connections: Connections::new(source, ports, Duration::from_secs(1))?,
            registration: Arc::new(registration_client::Factory::new(
                registration_client::Settings::new(launch),
            )?),
            primary: Primary {
                attachment: runtime.tablet_tx.clone(),
                status: runtime.status.clone(),
                listeners: Arc::new(listeners),
            },
            startups: JoinSet::new(),
        })
    }
    pub(super) fn progress(
        &mut self,
        slots: &mut Slots,
        extra: &ExtraSessionTemplate,
    ) -> Result<()> {
        let now = Instant::now();
        self.connections.poll(slots, now)?;
        loop {
            match self.connections.plan(slots, now)? {
                Decision::Start(admission) => self.start(admission, extra),
                _ => return Ok(()),
            }
        }
    }
    fn start(&mut self, admission: Admission, extra: &ExtraSessionTemplate) {
        let registration = self.registration.clone();
        if admission.instance() == 0 {
            let bindings = Bindings {
                attachment: self.primary.attachment.clone(),
                registration: Box::new(Bound::new(registration, self.primary.listeners.clone())),
                entropy: blent_config::credentials::system_entropy,
                work: Box::new(RuntimeWork::shared_status(self.primary.status.clone())),
            };
            self.startups
                .spawn(async move { (admission, Ok(bindings)) });
        } else {
            let instance = admission.instance();
            let template = extra.clone();
            self.startups.spawn(async move {
                let prepared = prepare_extra_session(&template, instance);
                let result = extra_bindings(prepared, template.shutdown_rx, registration).await;
                (admission, result)
            });
        }
    }
    pub(super) async fn next(
        &mut self,
    ) -> Option<std::result::Result<Started, tokio::task::JoinError>> {
        self.startups.join_next().await
    }
    pub(super) fn starting(&self) -> bool {
        !self.startups.is_empty()
    }
    pub(super) fn completed(
        &mut self,
        slots: &mut Slots,
        result: std::result::Result<Started, tokio::task::JoinError>,
    ) {
        match result {
            Ok((admission, Ok(bindings))) => self.connections.bind(slots, admission, bindings),
            Ok((admission, Err(error))) => {
                warn!("Could not start USB display: {error}");
                self.connections.failed(slots, admission, Instant::now());
            }
            // No proof of native cleanup: retain the portable startup ticket.
            Err(error) => warn!("USB display startup ownership quarantined: {error}"),
        }
    }
    pub(super) fn status(&self) -> Vec<runtime::TabletSession> {
        self.connections.status()
    }
    pub(super) async fn stop(&mut self, slots: &mut Slots) {
        self.connections.close(slots);
        while let Some(result) = self.next().await {
            self.completed(slots, result);
        }
        while !self.connections.retired() {
            if let Err(error) = self.connections.poll(slots, Instant::now()) {
                warn!("USB display cleanup: {error}");
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
async fn extra_bindings(
    prepared: Result<blent::session::Prepared>,
    shutdown: watch::Receiver<bool>,
    registration: Arc<registration_client::Factory>,
) -> Result<Bindings> {
    let (runtime, listeners) = prepared?.start_exported(shutdown).await?;
    Ok(Bindings {
        attachment: runtime.tablet_tx.clone(),
        registration: Box::new(Bound::new(registration, Arc::new(listeners))),
        entropy: blent_config::credentials::system_entropy,
        work: Box::new(RuntimeWork::owned(runtime)),
    })
}
impl Monitor {
    pub(super) fn usb_progress(&mut self) {
        let Some(usb) = &mut self.config.usb else {
            return;
        };
        if let Err(error) = usb.progress(&mut self.slots, &self.config.extra) {
            warn!("USB display connection: {error}");
        }
        self.preempt_for_usb();
    }
    pub(super) fn usb_completed(
        &mut self,
        result: Option<std::result::Result<Started, tokio::task::JoinError>>,
    ) {
        if let (Some(usb), Some(result)) = (&mut self.config.usb, result) {
            usb.completed(&mut self.slots, result);
        }
    }
}
pub(super) async fn event(usb: &mut Option<Native>, interval: &mut tokio::time::Interval) -> Event {
    let Some(usb) = usb else {
        return std::future::pending().await;
    };
    tokio::select! {
        _ = interval.tick() => Event::UsbTick,
        result = usb.next(), if usb.starting() => Event::UsbStartup(result),
    }
}
impl Monitor {
    pub(super) fn usb_event(&mut self, event: Event) -> Option<Event> {
        match event {
            Event::UsbStartup(result) => {
                self.usb_completed(result);
                None
            }
            Event::UsbTick => None,
            other => Some(other),
        }
    }
}

#[cfg(test)]
mod tests;
