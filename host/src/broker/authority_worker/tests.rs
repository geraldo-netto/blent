use super::*;
use crate::{
    broker::control::{Action, AuthorityKind, Controller, Device, Limits},
    secure_connection::IdentityRecord,
};
use blent_config::connection::{
    control::{Command, Notice},
    pairing::Ticket,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Condvar, Mutex,
};
use std::time::Duration;

#[derive(Default)]
struct Gate {
    entered: AtomicBool,
    released: Mutex<bool>,
    wake: Condvar,
}
impl Gate {
    fn wait(&self) {
        self.entered.store(true, Ordering::Release);
        let locked = self.released.lock().unwrap();
        drop(self.wake.wait_while(locked, |released| !*released).unwrap());
    }
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.wake.notify_all();
    }
}
struct Release(Arc<Gate>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}
struct Store {
    bytes: Option<Vec<u8>>,
    gate: Option<Arc<Gate>>,
    fail: bool,
}
impl TrustStorage for Store {
    fn load(&mut self) -> Result<Option<Vec<u8>>> {
        Ok(self.bytes.clone())
    }
    fn replace(&mut self, bytes: &[u8]) -> Result<()> {
        if let Some(gate) = self.gate.take() {
            gate.wait();
        }
        anyhow::ensure!(!self.fail, "injected write failure");
        self.bytes = Some(bytes.to_vec());
        Ok(())
    }
}
fn peer(n: u64) -> PeerId {
    PeerId::parse(format!("{n:064x}")).unwrap()
}
fn initialized(gate: Option<Arc<Gate>>, fail: bool) -> Initialized<Store> {
    let identity = Arc::new(Identity::load(IdentityRecord::generate().unwrap()).unwrap());
    let certificate = PeerCertificate::parse(identity.certificate().to_vec()).unwrap();
    let local = PeerId::parse(
        certificate
            .fingerprint()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
    .unwrap();
    let trust = Trust::open(
        Store {
            bytes: None,
            gate,
            fail,
        },
        local,
        64,
    )
    .unwrap();
    Initialized {
        identity,
        trust,
        identity_owner: Arc::new(()),
    }
}
async fn until(mut predicate: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
async fn event(worker: &mut Worker) -> Event {
    tokio::time::timeout(Duration::from_secs(3), worker.next())
        .await
        .unwrap()
        .unwrap()
}
async fn ready(worker: &mut Worker) -> Ready {
    match event(worker).await {
        Event::Ready(result) => result.unwrap(),
        _ => panic!("expected ready"),
    }
}
#[tokio::test]
async fn t853_explicit_worker_close_rejects_jobs_and_observes_thread_exit() {
    let mut worker =
        Worker::start(&Pool::new(1).unwrap(), || Ok(initialized(None, false))).unwrap();
    let identity = ready(&mut worker).await;
    let mut prepared = Prepared::new(identity.local, peer(1));
    let job = prepared.job.take().unwrap();
    let operation = job.operation().clone();
    worker.close();
    let returned = worker.submit(job).err().unwrap();
    assert_eq!(returned.operation(), &operation);
    until(|| worker.completion().is_complete()).await;
    assert!(worker.next().await.is_err());
    worker.close();
}
async fn reply(worker: &mut Worker) -> AuthorityReply {
    match event(worker).await {
        Event::Completed(reply) => reply,
        _ => panic!("expected completion"),
    }
}
struct Prepared {
    control: Controller<u64, u64>,
    client: u64,
    connection: u64,
    ticket: Ticket,
    job: Option<AuthorityJob>,
    _native: blent_config::retirement::Admission,
}
impl Prepared {
    fn new(local: PeerId, remote: PeerId) -> Self {
        let mut control = Controller::new(local, vec![], Limits::default()).unwrap();
        let client = control.admit_client().unwrap();
        control
            .replace_devices(vec![Device {
                key: 1,
                payload: 1,
                vendor: 1,
                product: 1,
                accessory: false,
            }])
            .unwrap();
        let (revision, target) = target(&control.snapshot());
        let connection = match control
            .command(client, Command::Connect { revision, target }, 0)
            .unwrap()
        {
            Action::Start { connection, .. } => connection,
            _ => panic!("expected start"),
        };
        let native = Pool::new(1).unwrap().reserve().unwrap();
        control.attach(connection, native.completion()).unwrap();
        control.identified(connection, remote.clone(), 1).unwrap();
        let ticket = ticket(&control.snapshot());
        control
            .command(
                client,
                Command::Decide {
                    connection,
                    ticket: ticket.clone(),
                    allow: true,
                    remember: true,
                },
                2,
            )
            .unwrap();
        let job = control
            .prepare_authentication(connection, &ticket, remote, 3)
            .unwrap();
        Self {
            control,
            client,
            connection,
            ticket,
            job: Some(job),
            _native: native,
        }
    }
    fn take_job(&mut self) -> AuthorityJob {
        self.job.take().unwrap()
    }
}
fn target(notices: &[Notice]) -> (u64, u64) {
    notices
        .iter()
        .find_map(|notice| match notice {
            Notice::Device {
                revision, target, ..
            } => Some((*revision, *target)),
            _ => None,
        })
        .unwrap()
}
fn ticket(notices: &[Notice]) -> Ticket {
    notices
        .iter()
        .find_map(|notice| match notice {
            Notice::Link { ticket, .. } => ticket.clone(),
            _ => None,
        })
        .unwrap()
}

#[tokio::test]
async fn t853_worker_initialization_error_is_reported_and_actually_retires() {
    let pool = Pool::new(1).unwrap();
    let mut worker =
        Worker::start::<Store>(&pool, || anyhow::bail!("injected initialization failure")).unwrap();
    let completion = worker.completion();
    match event(&mut worker).await {
        Event::Ready(result) => assert_eq!(
            result.err().unwrap().to_string(),
            "injected initialization failure"
        ),
        _ => panic!("expected ready failure"),
    }
    assert!(tokio::time::timeout(Duration::from_secs(3), worker.next())
        .await
        .unwrap()
        .is_err());
    until(|| completion.is_complete()).await;
    assert_eq!(pool.active(), 0);
}

#[tokio::test]
async fn t853_worker_full_queue_returns_exact_pairing_job() {
    let gate = Arc::new(Gate::default());
    let _release = Release(gate.clone());
    let pool = Pool::new(1).unwrap();
    let state = initialized(None, false);
    let local = public(&state).unwrap().local;
    let blocked = gate.clone();
    let worker = Worker::start(&pool, move || {
        blocked.wait();
        Ok(state)
    })
    .unwrap();
    until(|| gate.entered.load(Ordering::Acquire)).await;
    let mut first = Prepared::new(local.clone(), peer(1));
    let mut second = Prepared::new(local.clone(), peer(2));
    assert!(worker.submit(first.take_job()).is_ok());
    let job = second.take_job();
    let exact = job.operation().clone();
    let returned = worker.submit(job).err().unwrap();
    assert_eq!(returned.operation(), &exact);
    let mut trust = Trust::open(
        Store {
            bytes: None,
            gate: None,
            fail: false,
        },
        local,
        64,
    )
    .unwrap();
    let admitted = second
        .control
        .complete_authority(returned.run(&mut trust), 4)
        .unwrap()
        .unwrap();
    assert_eq!(admitted.lease.ticket, second.ticket);
    let completion = worker.completion();
    drop(worker);
    assert!(!completion.is_complete());
    assert!(pool.reserve().is_err());
    gate.release();
    until(|| completion.is_complete()).await;
}

#[tokio::test]
async fn t853_stalled_persistence_keeps_reactor_live_and_owner_until_thread_exit() {
    let gate = Arc::new(Gate::default());
    let _release = Release(gate.clone());
    let pool = Pool::new(1).unwrap();
    let store_gate = gate.clone();
    let mut worker =
        Worker::start(&pool, move || Ok(initialized(Some(store_gate), false))).unwrap();
    let state = ready(&mut worker).await;
    let mut pending = Prepared::new(state.local, peer(1));
    assert!(worker.submit(pending.take_job()).is_ok());
    until(|| gate.entered.load(Ordering::Acquire)).await;
    assert_eq!(
        pending.control.tick(120_001).unwrap(),
        vec![pending.connection]
    );
    assert!(pending.control.lease(pending.connection).is_none());
    tokio::task::yield_now().await;
    let completion = worker.completion();
    drop(worker);
    assert!(!completion.is_complete());
    assert!(pool.reserve().is_err());
    assert!(Worker::start(&pool, || Ok(initialized(None, false))).is_err());
    gate.release();
    until(|| completion.is_complete()).await;
    assert_eq!(pool.active(), 0);
}

#[tokio::test]
async fn t853_worker_authentication_and_revoke_replies_drive_controller() {
    let pool = Pool::new(1).unwrap();
    let mut worker = Worker::start(&pool, || Ok(initialized(None, false))).unwrap();
    let state = ready(&mut worker).await;
    assert!(state.saved.is_empty());
    assert!(!state.identity.certificate().is_empty());
    let owner = state.identity_owner.clone();
    assert!(Arc::strong_count(&owner) >= 2);
    let mut pending = Prepared::new(state.local, peer(1));
    assert!(worker.submit(pending.take_job()).is_ok());
    let result = reply(&mut worker).await;
    assert_eq!(result.operation().kind, AuthorityKind::Authenticate);
    let admitted = pending
        .control
        .complete_authority(result, 4)
        .unwrap()
        .unwrap();
    assert!(pending
        .control
        .pairing(pending.connection)
        .unwrap()
        .current(&admitted.lease));
    pending
        .control
        .command(pending.client, Command::Revoke { peer: peer(1) }, 5)
        .unwrap();
    let job = pending.control.next_authority().unwrap().unwrap();
    let operation = job.operation().clone();
    assert!(worker.submit(job).is_ok());
    let result = reply(&mut worker).await;
    assert_eq!(result.operation(), &operation);
    assert_eq!(operation.kind, AuthorityKind::Revoke);
    assert!(pending
        .control
        .complete_authority(result, 6)
        .unwrap()
        .is_none());
    assert!(pending.control.lease(pending.connection).is_none());
    let completion = worker.completion();
    drop(worker);
    until(|| completion.is_complete()).await;
}

#[tokio::test]
async fn t853_worker_storage_error_returns_original_pairing_fail_closed() {
    let pool = Pool::new(1).unwrap();
    let mut worker = Worker::start(&pool, || Ok(initialized(None, true))).unwrap();
    let state = ready(&mut worker).await;
    let mut pending = Prepared::new(state.local, peer(1));
    assert!(worker.submit(pending.take_job()).is_ok());
    let result = reply(&mut worker).await;
    assert!(pending.control.complete_authority(result, 4).is_err());
    assert_eq!(pending.control.closing(), vec![pending.connection]);
    assert!(pending.control.pairing(pending.connection).is_ok());
    assert!(pending.control.lease(pending.connection).is_none());
    let completion = worker.completion();
    drop(worker);
    until(|| completion.is_complete()).await;
}

#[tokio::test]
async fn t853_worker_ready_requires_healthy_store_and_reports_saved_peers() {
    for fail in [false, true] {
        let mut state = initialized(None, fail);
        let local = public(&state).unwrap().local;
        let mut pending = Prepared::new(local, peer(3));
        let _reply = pending.take_job().run(&mut state.trust);
        let pool = Pool::new(1).unwrap();
        let mut worker = Worker::start(&pool, move || Ok(state)).unwrap();
        match event(&mut worker).await {
            Event::Ready(Ok(ready)) => {
                assert!(!fail);
                assert_eq!(ready.saved, vec![peer(3)]);
            }
            Event::Ready(Err(error)) => {
                assert!(fail);
                assert_eq!(error.to_string(), "trust storage unavailable");
            }
            _ => panic!("expected initialization outcome"),
        }
        let completion = worker.completion();
        drop(worker);
        until(|| completion.is_complete()).await;
    }
}
