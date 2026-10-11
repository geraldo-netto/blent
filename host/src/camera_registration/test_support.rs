use super::*;
use crate::display_test_support::{peer, Backend, Calls, Route};
use blent_config::connection::{
    control::{Notice, State as LinkState},
    control_client::{
        Client as ControlClient, Factory as ControlFactory, Phase as ControlPhase,
        Status as ControlStatus, SubmitError,
    },
    control_view::{Action, Assembler, Limits, Model, View},
    pairing::Ticket,
};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

pub(crate) fn view(links: &[(u64, u64, LinkState)]) -> View {
    let mut assembler = Assembler::new(Limits::default()).unwrap();
    assembler
        .push(Notice::Begin {
            revision: 1,
            local: peer(1),
        })
        .unwrap();
    for &(connection, identity, state) in links {
        assembler
            .push(Notice::Link {
                revision: 1,
                connection,
                ticket: Some(Ticket {
                    epoch: 1,
                    peer: peer(identity),
                }),
                state,
                trusted: false,
            })
            .unwrap();
    }
    Model::default()
        .publish(
            assembler
                .push(Notice::End { revision: 1 })
                .unwrap()
                .unwrap(),
        )
        .unwrap()
}

pub(crate) struct Control {
    pub(crate) status: Mutex<ControlStatus>,
    pub(crate) closed: AtomicBool,
    pub(crate) done: AtomicBool,
    pub(crate) starts: AtomicUsize,
    pub(crate) fail: AtomicBool,
}
#[derive(Clone)]
struct ControlHandle(Arc<Control>);
impl std::ops::Deref for ControlHandle {
    type Target = Control;
    fn deref(&self) -> &Control {
        &self.0
    }
}
impl ControlClient for ControlHandle {
    fn status(&self) -> ControlStatus {
        self.status.lock().unwrap().clone()
    }
    fn submit(&self, _: Action) -> std::result::Result<u64, SubmitError> {
        Err(SubmitError::Closed)
    }
    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }
    fn retired(&self) -> bool {
        self.done.load(Ordering::SeqCst)
    }
}
impl ControlFactory for ControlHandle {
    fn start(&mut self) -> Result<Box<dyn ControlClient>> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        ensure!(
            !self.fail.load(Ordering::SeqCst),
            "unpublished control startup failure"
        );
        Ok(Box::new(self.clone()))
    }
}
pub(crate) struct Rig {
    pub(crate) source: Arc<Control>,
    pub(crate) calls: Arc<Mutex<Calls>>,
    pub(crate) now: Instant,
}
impl Rig {
    pub(crate) fn new(links: &[(u64, u64, LinkState)]) -> Self {
        Self {
            source: Arc::new(Control {
                status: Mutex::new(ControlStatus {
                    phase: ControlPhase::Ready,
                    view: Some(view(links)),
                    message: None,
                    outcome: None,
                }),
                closed: AtomicBool::new(false),
                done: AtomicBool::new(false),
                starts: AtomicUsize::new(0),
                fail: AtomicBool::new(false),
            }),
            calls: Arc::new(Mutex::new(Calls::default())),
            now: Instant::now(),
        }
    }
    pub(crate) fn owner(&self, profile: &CameraProfile, token: &str) -> Result<CameraRegistration> {
        CameraRegistration::new(
            profile,
            token,
            Discovery::new(
                Box::new(ControlHandle(self.source.clone())),
                Duration::from_secs(1),
                self.now,
            )?,
            Box::new(Backend(self.calls.clone())),
        )
    }
    pub(crate) fn client(&self) -> Arc<Route> {
        self.calls.lock().unwrap().clients[0].clone()
    }
    pub(crate) fn complete(&self) {
        self.client().set(Phase::Retired);
        self.client().done.store(true, Ordering::SeqCst);
        self.source.done.store(true, Ordering::SeqCst);
    }
}
pub(crate) fn profile() -> CameraProfile {
    CameraProfile {
        usb_peer: Some(peer(2).as_str().into()),
        ..Default::default()
    }
}
pub(crate) fn authenticated() -> Rig {
    Rig::new(&[(1, 2, LinkState::Authenticated)])
}
