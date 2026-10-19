//! One bounded persistence owner, independent of every media/control reactor.
use super::control::{AuthorityJob, AuthorityReply};
use crate::secure_connection::{Identity, PeerCertificate, RetainedIdentity};
use anyhow::{Context, Result};
use blent_config::{
    connection::{
        pairing::PeerId,
        trust::{Trust, TrustStorage},
    },
    retirement::{Admission, Completion, Pool},
    retirement_work::Thread,
};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct Initialized<S: TrustStorage> {
    pub identity: RetainedIdentity,
    pub trust: Trust<S>,
}
pub struct Ready {
    pub identity: Arc<Identity>,
    pub local: PeerId,
    pub saved: Vec<PeerId>,
}
pub enum Event {
    Ready(Result<Ready>),
    Completed(AuthorityReply),
}
pub struct Worker {
    jobs: Option<mpsc::Sender<AuthorityJob>>,
    events: mpsc::Receiver<Event>,
    completion: Completion,
}
impl Worker {
    /// Reuse the same process-wide pool for all attempts. A blocked initializer,
    /// transaction, destructor or join retains admission even after Worker drops.
    pub fn start<S: TrustStorage + Send + 'static>(
        pool: &Pool,
        initialize: impl FnOnce() -> Result<Initialized<S>> + Send + 'static,
    ) -> Result<Self> {
        Self::start_reserved(pool.reserve()?, initialize)
    }
    /// Transfer one original admission reserved before caller platform effects.
    /// It remains owned through initializer failure, destructor and actual join.
    pub fn start_reserved<S: TrustStorage + Send + 'static>(
        admission: Admission,
        initialize: impl FnOnce() -> Result<Initialized<S>> + Send + 'static,
    ) -> Result<Self> {
        let completion = admission.completion();
        let (jobs, input) = mpsc::channel(1);
        let (output, events) = mpsc::channel(1);
        let thread = std::thread::Builder::new()
            .name("blent-usb-trust".into())
            .spawn(move || run(initialize, input, output))?;
        admission.retain(Thread::new(thread));
        Ok(Self {
            jobs: Some(jobs),
            events,
            completion,
        })
    }
    /// Failed submission returns exact Pairing ownership to the dispatcher.
    pub fn submit(&self, job: AuthorityJob) -> std::result::Result<(), AuthorityJob> {
        match &self.jobs {
            Some(jobs) => jobs.try_send(job).map_err(|error| error.into_inner()),
            None => Err(job),
        }
    }
    /// Stop accepting jobs after the controller has consumed outstanding replies.
    /// Keep reading replies while draining; closing is not retirement evidence.
    pub fn close(&mut self) {
        self.jobs.take();
    }
    pub async fn next(&mut self) -> Result<Event> {
        self.events.recv().await.context("trust worker exited")
    }
    pub fn completion(&self) -> Completion {
        self.completion.clone()
    }
}
fn public<S: TrustStorage>(state: &Initialized<S>) -> Result<Ready> {
    let certificate = PeerCertificate::parse(state.identity.certificate().to_vec())?;
    let local = certificate.peer_id();
    Ok(Ready {
        identity: state.identity.shared(),
        local,
        saved: state.trust.approved_peers()?.to_vec(),
    })
}
fn run<S: TrustStorage>(
    initialize: impl FnOnce() -> Result<Initialized<S>>,
    mut input: mpsc::Receiver<AuthorityJob>,
    output: mpsc::Sender<Event>,
) {
    let mut state = match initialize() {
        Ok(state) => state,
        Err(error) => {
            let _ = output.blocking_send(Event::Ready(Err(error)));
            return;
        }
    };
    let ready = public(&state);
    let usable = ready.is_ok();
    if output.blocking_send(Event::Ready(ready)).is_err() || !usable {
        return;
    }
    while let Some(job) = input.blocking_recv() {
        let reply = job.run(&mut state.trust);
        if output.blocking_send(Event::Completed(reply)).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests;
