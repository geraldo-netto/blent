//! Pinned D-Bus owners; never use lock requests, idle hints or transport loss as state.
use super::*;
use futures_util::StreamExt;
use std::collections::HashMap;
use zbus::{
    message::{Sequence, Type},
    zvariant::{OwnedObjectPath, OwnedValue},
    Connection, MatchRule, Message, MessageStream,
};
const LOGIN: &str = "org.freedesktop.login1";
const MANAGER: &str = "org.freedesktop.login1.Manager";
const SESSION: &str = "org.freedesktop.login1.Session";
const LOGIN_PATH: &str = "/org/freedesktop/login1";
const PROPERTIES: &str = "org.freedesktop.DBus.Properties";
#[derive(Clone, Copy, Debug)]
pub(super) struct Target {
    pub name: &'static str,
    pub path: &'static str,
}
impl Target {
    pub fn desktop(desktop: &str) -> Option<Self> {
        let parts = desktop.split(':').collect::<Vec<_>>();
        if parts.iter().any(|part| {
            part.eq_ignore_ascii_case("cinnamon") || part.eq_ignore_ascii_case("x-cinnamon")
        }) {
            Some(Self {
                name: "org.cinnamon.Muffin.DisplayConfig",
                path: "/org/cinnamon/Muffin/DisplayConfig",
            })
        } else if parts.iter().any(|part| part.eq_ignore_ascii_case("gnome")) {
            Some(Self {
                name: "org.gnome.Mutter.DisplayConfig",
                path: "/org/gnome/Mutter/DisplayConfig",
            })
        } else {
            None
        }
    }
}
#[derive(Clone, Copy)]
pub(super) enum Patch {
    Power(Option<blent_config::host_activity::Power>),
    Lock(Option<bool>),
    Display(Option<bool>),
    Lost(Capabilities),
}
impl Patch {
    pub fn apply(self, mut state: HostState) -> HostState {
        match self {
            Self::Power(value) => state.power = value,
            Self::Lock(value) => state.locked = value,
            Self::Display(value) => state.display_on = value,
            Self::Lost(caps) => {
                if caps.power {
                    state.power = None;
                }
                if caps.lock {
                    state.locked = None;
                }
                if caps.display {
                    state.display_on = None;
                }
            }
        }
        state
    }
}
#[derive(Default, Debug)]
struct Flag {
    value: Option<bool>,
    floor: Sequence,
    initial_failed: bool,
}
impl Flag {
    fn set(&mut self, value: Option<bool>, position: Sequence) -> bool {
        if position <= self.floor {
            return false;
        }
        self.value = value;
        self.floor = position;
        true
    }
}
#[derive(Debug)]
struct Logind {
    session: Option<String>,
    sleep: Flag,
    shutdown: Flag,
    locked: Flag,
}
impl Logind {
    fn power(&self) -> Option<blent_config::host_activity::Power> {
        use blent_config::host_activity::Power;
        match (self.shutdown.value, self.sleep.value) {
            (Some(true), _) => Some(Power::Unavailable),
            (_, Some(true)) => Some(Power::Suspending),
            (Some(false), Some(false)) => Some(Power::Running),
            _ => None,
        }
    }
    fn signal(&mut self, message: &Message) -> Result<Option<Patch>> {
        let header = message.header();
        match (
            header.path().map(|p| p.as_str()),
            header.interface().map(|p| p.as_str()),
        ) {
            (Some(LOGIN_PATH), Some(MANAGER)) => self.power_signal(
                header.member().map(|m| m.as_str()),
                message,
                message.recv_position(),
            ),
            (Some(path), Some(PROPERTIES)) if self.session.as_deref() == Some(path) => {
                self.lock_signal(message)
            }
            _ => Ok(None),
        }
    }
    fn lock_signal(&mut self, message: &Message) -> Result<Option<Patch>> {
        if let Some(value) = changed(message, SESSION, "LockedHint")? {
            return Ok(self
                .locked
                .set(
                    value.map(bool::try_from).transpose()?,
                    message.recv_position(),
                )
                .then_some(Patch::Lock(self.locked.value)));
        }
        Ok(None)
    }
    fn power_signal(
        &mut self,
        member: Option<&str>,
        message: &Message,
        position: Sequence,
    ) -> Result<Option<Patch>> {
        let flag = match member {
            Some("PrepareForSleep") => &mut self.sleep,
            Some("PrepareForShutdown") => &mut self.shutdown,
            _ => return Ok(None),
        };
        let value: bool = payload(message)?;
        if flag.set(Some(value), position) {
            return Ok(Some(Patch::Power(self.power())));
        }
        Ok(None)
    }
}
#[derive(Debug)]
enum Kind {
    Logind(Logind),
    Display { target: Target, state: Flag },
}
#[derive(Debug)]
pub(super) struct Actor {
    bus: Connection,
    owner: String,
    signals: MessageStream,
    owners: MessageStream,
    kind: Kind,
    capabilities: Capabilities,
    state: HostState,
    deadline: Duration,
}
impl Actor {
    pub async fn logind(
        bus: Connection,
        session_id: Option<String>,
        capacity: usize,
        deadline: Duration,
    ) -> Result<Self> {
        let expires = tokio::time::Instant::now() + deadline;
        let (owner, signals, owners) =
            tokio::time::timeout_at(expires, streams(&bus, LOGIN, capacity)).await??;
        let data = logind_snapshot(&bus, &owner, session_id.as_deref(), expires).await;
        let capabilities = Capabilities {
            power: true,
            lock: data.session.is_some() || data.locked.initial_failed,
            display: false,
        };
        let state = HostState {
            power: data.power(),
            locked: data.locked.value,
            display_on: None,
        };
        Ok(Self {
            bus,
            owner,
            signals,
            owners,
            kind: Kind::Logind(data),
            capabilities,
            state,
            deadline,
        })
    }
    pub async fn display(
        bus: Connection,
        target: Target,
        capacity: usize,
        deadline: Duration,
    ) -> Result<Self> {
        let expires = tokio::time::Instant::now() + deadline;
        let (owner, signals, owners) =
            tokio::time::timeout_at(expires, streams(&bus, target.name, capacity)).await??;
        let observed = display_snapshot(&bus, &owner, target, expires).await?;
        let state = HostState {
            display_on: observed.value,
            ..HostState::default()
        };
        Ok(Self {
            bus,
            owner,
            signals,
            owners,
            kind: Kind::Display {
                target,
                state: observed,
            },
            capabilities: Capabilities {
                display: true,
                ..Capabilities::default()
            },
            state,
            deadline,
        })
    }
    pub fn include(&self, caps: &mut Capabilities, state: &mut HostState) -> bool {
        caps.power |= self.capabilities.power;
        caps.lock |= self.capabilities.lock;
        caps.display |= self.capabilities.display;
        if self.capabilities.power {
            state.power = self.state.power;
        }
        if self.capabilities.lock {
            state.locked = self.state.locked;
        }
        if self.capabilities.display {
            state.display_on = self.state.display_on;
        }
        !self.initial_query_failed()
    }
    pub async fn run(mut self, shared: Arc<Shared>) -> Result<()> {
        let outcome = std::panic::AssertUnwindSafe(self.watch(&shared))
            .catch_unwind()
            .await;
        if !matches!(outcome, Ok(Ok(()))) {
            shared.replacement.store(true, Ordering::Release);
            tracing::warn!(
                ?outcome,
                "Linux activity backend lost; original fields unknown"
            );
        }
        let _ = shared.update(Patch::Lost(self.capabilities));
        self.bus.close().await?;
        Ok(())
    }
    fn initial_query_failed(&self) -> bool {
        match &self.kind {
            Kind::Logind(data) => {
                data.sleep.initial_failed
                    || data.shutdown.initial_failed
                    || data.locked.initial_failed
            }
            Kind::Display { state, .. } => state.initial_failed,
        }
    }
    async fn watch(&mut self, shared: &Shared) -> Result<()> {
        loop {
            let wake = shared.wake.notified();
            tokio::pin!(wake);
            wake.as_mut().enable();
            if shared.stopped.load(Ordering::Acquire) {
                return Ok(());
            }
            ensure!(
                !self.initial_query_failed(),
                "initial activity property query failed"
            );
            tokio::select! { biased;
                _ = &mut wake => return Ok(()),
                owner = self.owners.next() => self.owner_change(owner.context("activity owner stream closed")??)?,
                signal = self.signals.next() => {
                    let signal = signal.context("activity signal stream closed")??;
                    self.apply_signal(&signal, shared).await?;
                }
            }
        }
    }
    async fn apply_signal(&mut self, message: &Message, shared: &Shared) -> Result<()> {
        let Some(patch) = self.signal(message)? else {
            return Ok(());
        };
        shared.update(patch)?;
        if matches!(patch, Patch::Lock(None) | Patch::Display(None)) {
            if let Some(fresh) = self.refresh().await? {
                shared.update(fresh)?;
            }
        }
        Ok(())
    }
    async fn refresh(&mut self) -> Result<Option<Patch>> {
        let (path, interface, name) = match &self.kind {
            Kind::Logind(data) => (
                data.session
                    .as_deref()
                    .context("lock observation has no session")?,
                SESSION,
                "LockedHint",
            ),
            Kind::Display { target, .. } => (target.path, target.name, "PowerSaveMode"),
        };
        let owner_name = match &self.kind {
            Kind::Logind(_) => LOGIN,
            Kind::Display { target, .. } => target.name,
        };
        let fresh = async {
            let snapshot = property(&self.bus, &self.owner, path, interface, name).await?;
            ensure!(
                original_owner(&self.bus, owner_name, &self.owner).await?,
                "original activity owner lost during query"
            );
            Ok::<_, anyhow::Error>(snapshot)
        };
        // Failed refresh must reach the retained replacement path. Unknown state
        // alone has no future event that guarantees another authoritative query.
        let (value, position) = tokio::time::timeout(self.deadline, fresh).await??;
        Ok(match &mut self.kind {
            Kind::Logind(data) => {
                let value = bool::try_from(value)?;
                data.locked
                    .set(Some(value), position)
                    .then_some(Patch::Lock(Some(value)))
            }
            Kind::Display { state, .. } => {
                let value = display_mode(i32::try_from(value)?)?;
                state.set(value, position).then_some(Patch::Display(value))
            }
        })
    }
    fn owner_change(&self, message: Message) -> Result<()> {
        let (_, previous, current): (String, String, String) = payload(&message)?;
        ensure!(
            previous != self.owner || current == self.owner,
            "original activity backend replaced"
        );
        Ok(())
    }
    fn signal(&mut self, message: &Message) -> Result<Option<Patch>> {
        if message.header().sender().map(|s| s.as_str()) != Some(self.owner.as_str()) {
            return Ok(None);
        }
        match &mut self.kind {
            Kind::Logind(data) => data.signal(message),
            Kind::Display { target, state } => display_signal(*target, state, message),
        }
    }
}
async fn display_snapshot(
    bus: &Connection,
    owner: &str,
    target: Target,
    expires: tokio::time::Instant,
) -> Result<Flag> {
    let query = async {
        let (value, floor) =
            property(bus, owner, target.path, target.name, "PowerSaveMode").await?;
        ensure!(
            original_owner(bus, target.name, owner).await?,
            "original display owner lost during initial query"
        );
        Ok::<_, anyhow::Error>((display_mode(i32::try_from(value)?)?, floor))
    };
    let snapshot = match tokio::time::timeout_at(expires, query).await {
        Ok(snapshot) => snapshot,
        Err(error) => Err(error.into()),
    };
    display_flag(snapshot)
}
fn display_flag(snapshot: Result<(Option<bool>, Sequence)>) -> Result<Flag> {
    match snapshot {
        Err(error) if retry_query(&error) => Ok(Flag {
            initial_failed: true,
            ..Flag::default()
        }),
        Err(error) => Err(error),
        Ok((value, floor)) => {
            ensure!(
                value.is_some(),
                "compositor display-power observation unsupported"
            );
            Ok(Flag {
                value,
                floor,
                initial_failed: false,
            })
        }
    }
}
fn payload<T>(message: &Message) -> Result<T>
where
    T: for<'de> serde::Deserialize<'de> + zbus::zvariant::Type,
{
    ensure!(
        message.body().len() <= 8192,
        "oversized native activity payload"
    );
    Ok(message.body().deserialize()?)
}
async fn original_owner(bus: &Connection, name: &str, owner: &str) -> Result<bool> {
    let reply = bus
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetNameOwner",
            &(name,),
        )
        .await?;
    let current: zbus::names::OwnedUniqueName = payload(&reply)?;
    Ok(current.as_str() == owner)
}
use anyhow::Context;
async fn streams(
    bus: &Connection,
    name: &str,
    capacity: usize,
) -> Result<(String, MessageStream, MessageStream)> {
    // Subscribe to loss before resolving the owner, then pin every state call and
    // signal filter to that exact unique name. Replacement never resumes it.
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender("org.freedesktop.DBus")?
        .interface("org.freedesktop.DBus")?
        .member("NameOwnerChanged")?
        .add_arg(name)?
        .build();
    let owners = MessageStream::for_match_rule(rule, bus, Some(capacity)).await?;
    let reply = bus
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetNameOwner",
            &(name,),
        )
        .await?;
    let owner: zbus::names::OwnedUniqueName = payload(&reply)?;
    let owner = owner.to_string();
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(owner.as_str())?
        .build();
    let signals = MessageStream::for_match_rule(rule, bus, Some(capacity)).await?;
    Ok((owner, signals, owners))
}
async fn property(
    bus: &Connection,
    owner: &str,
    path: &str,
    interface: &str,
    name: &str,
) -> Result<(OwnedValue, Sequence)> {
    let reply = bus
        .call_method(
            Some(owner),
            path,
            Some(PROPERTIES),
            "Get",
            &(interface, name),
        )
        .await?;
    Ok((payload(&reply)?, reply.recv_position()))
}
fn retry_query(error: &anyhow::Error) -> bool {
    match error.downcast_ref::<zbus::Error>() {
        Some(zbus::Error::MethodError(name, _, _)) => !matches!(
            name.as_str(),
            "org.freedesktop.DBus.Error.UnknownProperty"
                | "org.freedesktop.DBus.Error.UnknownInterface"
                | "org.freedesktop.DBus.Error.UnknownMethod"
        ),
        _ => true,
    }
}
async fn boolean(bus: &Connection, owner: &str, path: &str, interface: &str, name: &str) -> Flag {
    match property(bus, owner, path, interface, name).await {
        Ok((value, floor)) => {
            let value = bool::try_from(value).ok();
            Flag {
                initial_failed: value.is_none(),
                value,
                floor,
            }
        }
        Err(error) => Flag {
            initial_failed: retry_query(&error),
            ..Flag::default()
        },
    }
}
async fn logind_snapshot(
    bus: &Connection,
    owner: &str,
    id: Option<&str>,
    expires: tokio::time::Instant,
) -> Logind {
    match tokio::time::timeout_at(expires, logind_state(bus, owner, id)).await {
        Ok(Ok(data)) => data,
        _ => Logind {
            session: None,
            sleep: Flag::default(),
            shutdown: Flag::default(),
            locked: Flag {
                initial_failed: true,
                ..Flag::default()
            },
        },
    }
}
async fn logind_state(bus: &Connection, owner: &str, id: Option<&str>) -> Result<Logind> {
    let sleep = boolean(bus, owner, LOGIN_PATH, MANAGER, "PreparingForSleep").await;
    let shutdown = boolean(bus, owner, LOGIN_PATH, MANAGER, "PreparingForShutdown").await;
    let session = session_path(bus, owner, id).await?;
    let locked = match &session {
        Some(path) => boolean(bus, owner, path, SESSION, "LockedHint").await,
        None => Flag::default(),
    };
    ensure!(
        original_owner(bus, LOGIN, owner).await?,
        "original logind owner lost during initial query"
    );
    Ok(Logind {
        session,
        sleep,
        shutdown,
        locked,
    })
}
async fn session_path(bus: &Connection, owner: &str, id: Option<&str>) -> Result<Option<String>> {
    let Some(reply) = session_reply(bus, owner, id).await? else {
        return Ok(None);
    };
    let path = session_object(&reply)?;
    let (value, _) = property(bus, owner, path.as_str(), SESSION, "User").await?;
    own_session(path, value)
}
fn session_object(reply: &Message) -> Result<OwnedObjectPath> {
    let path: OwnedObjectPath = payload(reply)?;
    ensure!(
        path.as_str()
            .starts_with("/org/freedesktop/login1/session/")
            && path.as_str().len() <= 256,
        "invalid logind session path"
    );
    Ok(path)
}
fn own_session(path: OwnedObjectPath, value: OwnedValue) -> Result<Option<String>> {
    let (uid, _): (u32, OwnedObjectPath) = value.try_into()?;
    Ok((uid == unsafe { libc::geteuid() }).then(|| path.to_string()))
}
async fn session_reply(bus: &Connection, owner: &str, id: Option<&str>) -> Result<Option<Message>> {
    let by_pid = bus
        .call_method(
            Some(owner),
            LOGIN_PATH,
            Some(MANAGER),
            "GetSessionByPID",
            &(std::process::id(),),
        )
        .await;
    if let Some(reply) = session_lookup(by_pid)? {
        return Ok(Some(reply));
    }
    let Some(id) = id.filter(|id| !id.is_empty() && id.len() <= 128 && !id.contains('/')) else {
        return Ok(None);
    };
    session_lookup(
        bus.call_method(Some(owner), LOGIN_PATH, Some(MANAGER), "GetSession", &(id,))
            .await,
    )
}
fn session_lookup(reply: zbus::Result<Message>) -> Result<Option<Message>> {
    match reply {
        Ok(reply) => Ok(Some(reply)),
        Err(zbus::Error::MethodError(name, _, _))
            if matches!(
                name.as_str(),
                "org.freedesktop.login1.NoSessionForPID"
                    | "org.freedesktop.login1.NoSuchSession"
                    | "org.freedesktop.DBus.Error.UnknownMethod"
                    | "org.freedesktop.DBus.Error.UnknownInterface"
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

fn changed(
    message: &Message,
    interface: &str,
    property: &str,
) -> Result<Option<Option<OwnedValue>>> {
    if message.header().member().map(|m| m.as_str()) != Some("PropertiesChanged") {
        return Ok(None);
    }
    let (observed, mut values, invalid): (String, HashMap<String, OwnedValue>, Vec<String>) =
        payload(message)?;
    ensure!(
        values.len() <= 64 && invalid.len() <= 64,
        "oversized native property changes"
    );
    if observed != interface {
        return Ok(None);
    }
    if invalid.iter().any(|name| name == property) {
        return Ok(Some(None));
    }
    Ok(values.remove(property).map(Some))
}
fn display_mode(mode: i32) -> Result<Option<bool>> {
    match mode {
        -1 => Ok(None),
        0 => Ok(Some(true)),
        1..=3 => Ok(Some(false)),
        _ => anyhow::bail!("invalid compositor power mode"),
    }
}
fn display_signal(target: Target, state: &mut Flag, message: &Message) -> Result<Option<Patch>> {
    let header = message.header();
    if header.path().map(|p| p.as_str()) != Some(target.path)
        || header.interface().map(|i| i.as_str()) != Some(PROPERTIES)
    {
        return Ok(None);
    }
    if let Some(value) = changed(message, target.name, "PowerSaveMode")? {
        let value = value
            .map(i32::try_from)
            .transpose()?
            .map(display_mode)
            .transpose()?
            .flatten();
        return Ok(state
            .set(value, message.recv_position())
            .then_some(Patch::Display(state.value)));
    }
    Ok(None)
}

#[cfg(test)]
mod tests;
