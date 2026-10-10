//! Producer-owned attachment epochs. Never infer a replacement from a coalescing
//! presence watch; invalidate metadata synchronously before forwarding/launch.
use crate::media::EncoderSettings;
use blent_config::connection::routes::{Identity, Link};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

mod adb;
mod auth;
mod registered;
use auth::Authentication;
pub use registered::RegisteredRoute;

struct State {
    authentication: Authentication,
    legacy: Authentication,
    registered: bool,
    identity: Option<Identity>,
    generation: u64,
    transport: Option<Link>,
    closed: bool,
}
struct Shared {
    state: Mutex<State>,
    presence: watch::Sender<bool>,
    generation: watch::Sender<u64>,
    settings: watch::Sender<EncoderSettings>,
}
#[derive(Clone)]
pub struct Attachment(Arc<Shared>);
impl Attachment {
    pub fn new(settings: watch::Sender<EncoderSettings>) -> Self {
        Self(Arc::new(Shared {
            state: Mutex::new(State {
                authentication: Authentication::Unmanaged,
                legacy: Authentication::Unmanaged,
                registered: false,
                identity: None,
                generation: 0,
                transport: None,
                closed: false,
            }),
            presence: watch::channel(false).0,
            generation: watch::channel(0).0,
            settings,
        }))
    }

    pub fn with_token(settings: watch::Sender<EncoderSettings>, token: Option<String>) -> Self {
        let attachment = Self::new(settings);
        {
            let mut state = attachment.0.state.lock().unwrap();
            state.authentication = Authentication::configured(token);
            state.legacy = state.authentication.renewable();
        }
        attachment
    }

    pub fn token(&self) -> anyhow::Result<Option<String>> {
        self.0
            .state
            .lock()
            .unwrap()
            .authentication
            .expected(None)
            .map(|token| token.map(str::to_owned))
    }

    /// Begin a transport handoff before its first authenticated message can
    /// arrive. Proven same-tablet migration retains geometry; replacement or
    /// absence invalidates it. Every handoff retires earlier control leases.
    pub fn begin_route(&self, identity: Option<Identity>, transport: Option<Link>) {
        let mut state = self.0.state.lock().unwrap();
        if state.closed {
            return;
        }
        let preserve = identity.is_some() && state.identity == identity;
        if state.registered {
            if self.advance_registered(&mut state).is_err() {
                return;
            }
        } else {
            state.generation = state.generation.wrapping_add(1);
        }
        state.observed_authentication(&identity, preserve);
        state.transport = transport.filter(|_| identity.is_some());
        state.identity = identity;
        if !preserve {
            self.reset_metadata();
        }
        self.0.generation.send_replace(state.generation);
        self.0.presence.send_replace(false);
    }

    /// Permanently retire this session. Transport polling cannot revive it.
    /// Retrying requires a new session owner and attachment.
    pub fn retire(&self) {
        // A worker action may have panicked while holding the generation lock.
        // Recover only to revoke all authority, never to resume partial state.
        let mut state = self
            .0
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if state.closed {
            return;
        }
        state.closed = true;
        self.0.state.clear_poison();
        state.authentication = Authentication::Unavailable;
        state.identity = None;
        state.transport = None;
        state.generation = state.generation.wrapping_add(1);
        self.reset_metadata();
        self.0.generation.send_replace(state.generation);
        self.0.presence.send_replace(false);
    }
    fn reset_metadata(&self) {
        self.0.settings.send_if_modified(|settings| {
            let ready = settings.geometry_ready
                || settings.decoders.is_some()
                || settings.selection.is_some();
            settings.clear_decoders();
            settings.geometry_ready = false;
            ready
        });
    }

    /// Publish completed forwarding, or fully detach. Keep the familiar watch
    /// sender result so callers can ignore shutdown receivers disappearing.
    pub fn send(&self, present: bool) -> Result<(), watch::error::SendError<bool>> {
        if !present {
            self.begin_route(None, None);
        }
        let state = self.0.state.lock().unwrap();
        if state.closed {
            return Err(watch::error::SendError(present));
        }
        self.0.presence.send(present)
    }

    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.0.presence.subscribe()
    }

    /// Capture at accept time, before WebSocket/authentication can await.
    pub fn lease(&self) -> Lease {
        let state = self.0.state.lock().unwrap();
        self.lease_locked(&state)
    }

    fn lease_locked(&self, state: &State) -> Lease {
        Lease {
            authentication: state.authentication.clone(),
            identity: state.identity.clone(),
            attachment: self.clone(),
            generation: state.generation,
            transport: state.transport,
            closed: state.closed,
            changed: self.0.generation.subscribe(),
        }
    }
}

pub struct Lease {
    #[cfg_attr(feature = "inproc-encoder", allow(dead_code))]
    identity: Option<Identity>,
    authentication: Authentication,
    attachment: Attachment,
    generation: u64,
    transport: Option<Link>,
    changed: watch::Receiver<u64>,
    closed: bool,
}
impl Lease {
    /// Identity and route are captured atomically with the attachment generation.
    pub fn profile_identity(&self) -> Option<(String, &'static str)> {
        let identity = self.identity.as_ref()?.profile()?;
        Some((identity.to_string(), self.transport()?))
    }

    pub fn token<'a>(&'a self, fallback: Option<&'a str>) -> anyhow::Result<Option<&'a str>> {
        self.authentication.expected(fallback)
    }

    /// Poll video I/O under the generation check. Retirement cannot race a
    /// synchronous socket write; a pending poll never holds the mutex asleep.
    pub async fn run(
        &self,
        work: impl std::future::Future<Output = anyhow::Result<()>>,
    ) -> anyhow::Result<()> {
        let mut work = std::pin::pin!(work);
        let checked = std::future::poll_fn(|cx| {
            let mut result = std::task::Poll::Ready(Ok(()));
            self.apply(|| result = work.as_mut().poll(cx));
            result
        });
        let mut retirement = self.retirement();
        tokio::select! {
            biased;
            _ = retirement.retired() => Ok(()),
            result = checked => result,
        }
    }

    /// Immutable accepted route: never read a replacement tablet's transport.
    pub fn transport(&self) -> Option<&'static str> {
        self.transport.map(Link::profile_name)
    }

    /// Serialize check and action with invalidation. An old socket cannot
    /// restore geometry or inject contacts after a new attachment begins.
    pub fn apply(&self, action: impl FnOnce()) -> bool {
        let state = self.attachment.0.state.lock().unwrap();
        if state.closed || state.generation != self.generation {
            return false;
        }
        action();
        true
    }
    pub fn retirement(&self) -> Retirement {
        Retirement {
            changed: self.changed.clone(),
            generation: self.generation,
            closed: self.closed,
        }
    }
}
pub struct Retirement {
    changed: watch::Receiver<u64>,
    generation: u64,
    closed: bool,
}
impl Retirement {
    pub async fn retired(&mut self) {
        if !self.closed && *self.changed.borrow() == self.generation {
            let _ = self.changed.changed().await;
        }
    }
}
