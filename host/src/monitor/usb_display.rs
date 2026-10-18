//! Linux daemon composition. Portable connection policy owns no native handles.
use super::*;
use blent::{
    broker::linux::{
        client::Launch, display_registration::Bound, presentation, registration_client,
    },
    connection_discovery::Discovery,
    display_connections::{Admission, Connections, Decision},
    registered_session::{ActivityBinding, Bindings},
    session_network::RetainedListeners,
    session_work::RuntimeWork,
};

type Started = (Admission, Result<Bindings>);
type StartupProof = blent::display_connections::Recovery;
struct Primary {
    activity: ActivityBinding,
    attachment: attachment::Attachment,
    status: watch::Receiver<blent::session_state::State>,
    listeners: Arc<RetainedListeners>,
}
pub(crate) struct Native {
    activity: blent::activity_view::View,
    connections: Connections,
    registration: Arc<registration_client::Factory>,
    primary: Primary,
    startups: JoinSet<Started>,
    shared_startup: blent_config::retirement::Pool,
    startup_proofs: std::collections::HashMap<tokio::task::Id, StartupProof>,
    returned_proofs: Vec<StartupProof>,
    abandoned: Vec<StartupProof>,
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
            activity: blent::activity_view::View::new(
                blent_config::host_activity::ActivityTarget::Display,
                config.extra.activity.clone(),
            ),
            connections: Connections::new(source, ports, Duration::from_secs(1))?,
            registration: Arc::new(registration_client::Factory::new(
                registration_client::Settings::new(launch),
            )?),
            primary: Primary {
                activity: activity_binding(runtime, config.extra.activity.clone())?,
                attachment: runtime.tablet_tx.clone(),
                status: runtime.status.clone(),
                listeners: Arc::new(listeners),
            },
            startups: JoinSet::new(),
            shared_startup: blent_config::retirement::Pool::new(1)?,
            startup_proofs: Default::default(),
            returned_proofs: vec![],
            abandoned: vec![],
        })
    }
    pub(super) fn progress(
        &mut self,
        slots: &mut Slots,
        extra: &ExtraSessionTemplate,
    ) -> Result<()> {
        let now = Instant::now();
        self.reap_startups(slots, now);
        self.connections.poll(slots, now)?;
        loop {
            match self.connections.plan(slots, now)? {
                Decision::Start(admission) => self.start(slots, admission, extra),
                _ => return Ok(()),
            }
        }
    }
    fn start(&mut self, slots: &mut Slots, admission: Admission, extra: &ExtraSessionTemplate) {
        if admission.instance() == 0 {
            self.start_shared(slots, admission);
            return;
        }
        let owner = match blent::session::Workers::reserve() {
            Ok(owner) => owner,
            Err(error) => {
                warn!("Could not reserve USB display startup: {error}");
                self.connections.failed(slots, admission, Instant::now());
                return;
            }
        };
        let registration = self.registration.clone();
        let instance = admission.instance();
        let template = extra.clone();
        self.spawn(admission, owner, move |owner| async move {
            let prepared = prepare_extra_session(&template, instance);
            extra_bindings(prepared, template.shutdown_rx, registration, owner).await
        });
    }
    fn start_shared(&mut self, slots: &mut Slots, admission: Admission) {
        // The primary reuses an existing runtime, not a second native session.
        // Retain only its bounded unpublished binding task until actual join.
        let owner = match self.shared_startup.reserve() {
            Ok(owner) => owner,
            Err(error) => {
                warn!("Could not reserve shared USB startup: {error}");
                self.connections.failed(slots, admission, Instant::now());
                return;
            }
        };
        let bindings = Bindings {
            activity: Some(self.primary.activity.clone()),
            attachment: self.primary.attachment.clone(),
            registration: Box::new(Bound::new(
                self.registration.clone(),
                self.primary.listeners.clone(),
            )),
            entropy: blent_config::credentials::system_entropy,
            work: Box::new(RuntimeWork::shared_status(self.primary.status.clone())),
        };
        self.track(admission, owner.completion(), async move {
            let _owner = owner;
            Ok(bindings)
        });
    }
    fn spawn<F, W>(
        &mut self,
        admission: Admission,
        owner: blent::session::SessionAdmission,
        work: W,
    ) where
        F: std::future::Future<Output = Result<Bindings>> + Send + 'static,
        W: FnOnce(blent::session::SessionAdmission) -> F + Send + 'static,
    {
        self.track(
            admission,
            owner.completion(),
            async move { work(owner).await },
        );
    }
    fn track<F>(
        &mut self,
        admission: Admission,
        completion: blent_config::retirement::Completion,
        work: F,
    ) where
        F: std::future::Future<Output = Result<Bindings>> + Send + 'static,
    {
        let proof = admission.recovery(completion);
        let task = self.startups.spawn(async move { (admission, work.await) });
        self.startup_proofs.insert(task.id(), proof);
    }
    pub(super) async fn next(
        &mut self,
    ) -> Option<std::result::Result<Started, tokio::task::JoinError>> {
        let result = self.startups.join_next_with_id().await?;
        Some(match result {
            Ok((id, started)) => {
                if let Some(proof) = self.startup_proofs.remove(&id) {
                    self.returned_proofs.push(proof);
                }
                Ok(started)
            }
            Err(error) => {
                if let Some(proof) = self.startup_proofs.remove(&error.id()) {
                    self.abandoned.push(proof);
                }
                Err(error)
            }
        })
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
            Ok((admission, result)) => self.complete_start(slots, admission, result),
            Err(error) => {
                warn!("USB display startup exited; actual cleanup still required: {error}")
            }
        }
        self.reap_startups(slots, Instant::now());
    }
    fn take_proof(&mut self, admission: &Admission) -> Option<StartupProof> {
        let index = self
            .returned_proofs
            .iter()
            .position(|proof| proof.matches(admission))?;
        Some(self.returned_proofs.swap_remove(index))
    }
    fn complete_start(
        &mut self,
        slots: &mut Slots,
        admission: Admission,
        result: Result<Bindings>,
    ) {
        let proof = self.take_proof(&admission);
        match result {
            Ok(bindings) => self.connections.bind(slots, admission, bindings),
            Err(error) => {
                warn!("Could not start USB display: {error}");
                if let Some(proof) = proof {
                    self.abandoned.push(proof);
                } else {
                    self.connections.failed(slots, admission, Instant::now());
                }
            }
        }
    }
    fn reap_startups(&mut self, slots: &mut Slots, now: Instant) {
        self.abandoned
            .retain(|proof| !self.connections.recover_failed(slots, proof, now));
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
            self.reap_startups(slots, Instant::now());
            if let Err(error) = self.connections.poll(slots, Instant::now()) {
                warn!("USB display cleanup: {error}");
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
fn activity_binding(
    runtime: &ExtraSession,
    source: Option<watch::Receiver<blent_config::host_activity::Publication>>,
) -> Result<ActivityBinding> {
    Ok(ActivityBinding {
        control: runtime
            .quiescence
            .clone()
            .ok_or_else(|| anyhow::anyhow!("native display quiescence unsupported"))?,
        retirement: runtime.retirement.clone(),
        source,
    })
}
async fn extra_bindings(
    prepared: Result<blent::session::Prepared>,
    shutdown: watch::Receiver<bool>,
    registration: Arc<registration_client::Factory>,
    owner: blent::session::SessionAdmission,
) -> Result<Bindings> {
    let prepared = prepared?;
    let source = prepared.activity.clone();
    let (runtime, listeners) = prepared.start_exported_reserved(shutdown, owner).await?;
    Ok(Bindings {
        activity: Some(activity_binding(&runtime, source)?),
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
    let activity = usb.activity.changed();
    tokio::select! {
        _ = activity => Event::UsbTick,
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
