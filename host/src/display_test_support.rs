use crate::{connection_discovery::Candidate, registered_session::Work};
use crate::{connection_discovery::Discovery, media::EncoderSettings};
use anyhow::{ensure, Result};
use blent_config::connection::{
    control::{Notice, State as LinkState},
    control_client::{
        Client as ControlClient, Factory, Phase as ControlPhase, Status as ControlStatus,
        SubmitError,
    },
    control_view::{Action, Assembler, Limits, Model, View},
    pairing::{PeerId, Ticket as PairingTicket},
    registration_client::{
        Client, Factory as Registration, Phase, Request as RegistrationRequest, Status,
    },
    routes::RouteId,
};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

pub(crate) fn peer(id: u64) -> PeerId {
    PeerId::parse(format!("{id:064x}")).unwrap()
}
struct Control(View);
impl ControlClient for Control {
    fn status(&self) -> ControlStatus {
        ControlStatus {
            phase: ControlPhase::Ready,
            view: Some(self.0.clone()),
            message: None,
            outcome: None,
        }
    }
    fn submit(&self, _: Action) -> std::result::Result<u64, SubmitError> {
        Err(SubmitError::Closed)
    }
    fn close(&self) {}
    fn retired(&self) -> bool {
        false
    }
}
impl Factory for Control {
    fn start(&mut self) -> Result<Box<dyn ControlClient>> {
        Ok(Box::new(Self(self.0.clone())))
    }
}
pub(crate) fn candidate() -> Candidate {
    let mut assembler = Assembler::new(Limits::default()).unwrap();
    assembler
        .push(Notice::Begin {
            revision: 1,
            local: peer(1),
        })
        .unwrap();
    assembler
        .push(Notice::Link {
            revision: 1,
            connection: 1,
            ticket: Some(PairingTicket {
                epoch: 1,
                peer: peer(2),
            }),
            state: LinkState::Authenticated,
            trusted: false,
        })
        .unwrap();
    let view = Model::default()
        .publish(
            assembler
                .push(Notice::End { revision: 1 })
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    let now = Instant::now();
    Discovery::new(Box::new(Control(view)), Duration::from_secs(1), now)
        .unwrap()
        .poll(now)
        .unwrap()
        .remove(0)
}
#[derive(Default)]
pub(crate) struct Native {
    pub(crate) failed: AtomicBool,
    pub(crate) done: AtomicBool,
    pub(crate) closes: AtomicUsize,
}
impl Work for Arc<Native> {
    fn alive(&self) -> bool {
        !self.failed.load(Ordering::SeqCst)
    }
    fn close(&mut self) {
        self.closes.fetch_add(1, Ordering::SeqCst);
    }
    fn retired(&self) -> bool {
        self.done.load(Ordering::SeqCst)
    }
}
pub(crate) struct Route {
    pub(crate) status: Mutex<Status>,
    pub(crate) done: AtomicBool,
    pub(crate) closed: AtomicBool,
}
impl Route {
    pub(crate) fn set(&self, phase: Phase) {
        *self.status.lock().unwrap() = Status {
            phase,
            route: Some(RouteId {
                owner: 1,
                revision: 1,
            }),
            remote_retired: phase == Phase::Retired,
            failure: None,
        };
    }
}
pub(crate) struct Stub(pub(crate) Arc<Route>);
impl Client for Stub {
    fn status(&self) -> Status {
        self.0.status.lock().unwrap().clone()
    }
    fn close(&self) {
        self.0.closed.store(true, Ordering::SeqCst);
    }
    fn retired(&self) -> bool {
        self.0.done.load(Ordering::SeqCst)
    }
}
#[derive(Default)]
pub(crate) struct Calls {
    pub(crate) requests: Vec<RegistrationRequest>,
    pub(crate) clients: Vec<Arc<Route>>,
    pub(crate) fail: bool,
}
pub(crate) struct Backend(pub(crate) Arc<Mutex<Calls>>);
impl Registration for Backend {
    fn start(&mut self, request: RegistrationRequest) -> Result<Box<dyn Client>> {
        let mut calls = self.0.lock().unwrap();
        calls.requests.push(request);
        ensure!(!calls.fail, "T925 unpublished registration failure");
        let client = Arc::new(Route {
            status: Mutex::new(Status {
                phase: Phase::Starting,
                route: None,
                remote_retired: false,
                failure: None,
            }),
            done: AtomicBool::new(false),
            closed: AtomicBool::new(false),
        });
        calls.clients.push(client.clone());
        Ok(Box::new(Stub(client)))
    }
}
pub(crate) fn entropy(bytes: &mut [u8]) -> Result<()> {
    static NEXT: AtomicU8 = AtomicU8::new(1);
    bytes.fill(NEXT.fetch_add(1, Ordering::SeqCst));
    Ok(())
}
pub(crate) fn settings() -> EncoderSettings {
    EncoderSettings {
        encoder: "libx264".into(),
        fps: 30,
        bitrate: 20000,
        width: 640,
        height: 480,
        quality: 18,
        width_mm: 310,
        height_mm: 194,
        stream_scale: 1,
        geometry_ready: false,
        decoders: None,
        decoder_epoch: 0,
        selection: None,
    }
}
