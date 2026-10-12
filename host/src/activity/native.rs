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
        let (owner, signals, owners) = streams(&bus, LOGIN, capacity).await?;
        let sleep = boolean(&bus, &owner, LOGIN_PATH, MANAGER, "PreparingForSleep").await;
        let shutdown = boolean(&bus, &owner, LOGIN_PATH, MANAGER, "PreparingForShutdown").await;
        let session = session_path(&bus, &owner, session_id.as_deref()).await;
        let locked = match &session {
            Some(path) => boolean(&bus, &owner, path, SESSION, "LockedHint").await,
            None => Flag::default(),
        };
        let data = Logind {
            session,
            sleep,
            shutdown,
            locked,
        };
        let capabilities = Capabilities {
            power: true,
            lock: data.session.is_some(),
            display: false,
        };
        let state = if original_owner(&bus, LOGIN, &owner).await.unwrap_or(false) {
            HostState {
                power: data.power(),
                locked: data.locked.value,
                display_on: None,
            }
        } else {
            HostState::default()
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
        let (owner, signals, owners) = streams(&bus, target.name, capacity).await?;
        let (value, floor) =
            property(&bus, &owner, target.path, target.name, "PowerSaveMode").await?;
        let value = display_mode(i32::try_from(value)?)?;
        ensure!(
            value.is_some(),
            "compositor display-power observation unsupported"
        );
        let state = HostState {
            display_on: if original_owner(&bus, target.name, &owner)
                .await
                .unwrap_or(false)
            {
                value
            } else {
                None
            },
            ..HostState::default()
        };
        Ok(Self {
            bus,
            owner,
            signals,
            owners,
            kind: Kind::Display {
                target,
                state: Flag { value, floor },
            },
            capabilities: Capabilities {
                display: true,
                ..Capabilities::default()
            },
            state,
            deadline,
        })
    }
    pub fn include(&self, caps: &mut Capabilities, state: &mut HostState) {
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
    }
    pub async fn run(mut self, shared: Arc<Shared>) -> Result<()> {
        let outcome = std::panic::AssertUnwindSafe(self.watch(&shared))
            .catch_unwind()
            .await;
        if !matches!(outcome, Ok(Ok(()))) {
            tracing::warn!(
                ?outcome,
                "Linux activity backend lost; original fields unknown"
            );
        }
        let _ = shared.update(Patch::Lost(self.capabilities));
        self.bus.close().await?;
        Ok(())
    }
    async fn watch(&mut self, shared: &Shared) -> Result<()> {
        loop {
            let wake = shared.wake.notified();
            tokio::pin!(wake);
            wake.as_mut().enable();
            if shared.stopped.load(Ordering::Acquire) {
                return Ok(());
            }
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
            if let Some(fresh) = self.refresh().await {
                shared.update(fresh)?;
            }
        }
        Ok(())
    }
    async fn refresh(&mut self) -> Option<Patch> {
        let (path, interface, name) = match &self.kind {
            Kind::Logind(data) => (data.session.as_deref()?, SESSION, "LockedHint"),
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
        let (value, position) = tokio::time::timeout(self.deadline, fresh)
            .await
            .ok()?
            .ok()?;
        match &mut self.kind {
            Kind::Logind(data) => {
                let value = bool::try_from(value).ok()?;
                data.locked
                    .set(Some(value), position)
                    .then_some(Patch::Lock(Some(value)))
            }
            Kind::Display { state, .. } => {
                let value = display_mode(i32::try_from(value).ok()?).ok()?;
                state.set(value, position).then_some(Patch::Display(value))
            }
        }
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
async fn boolean(bus: &Connection, owner: &str, path: &str, interface: &str, name: &str) -> Flag {
    match property(bus, owner, path, interface, name).await {
        Ok((value, floor)) => Flag {
            value: bool::try_from(value).ok(),
            floor,
        },
        Err(_) => Flag::default(),
    }
}
async fn session_path(bus: &Connection, owner: &str, id: Option<&str>) -> Option<String> {
    let by_pid = bus
        .call_method(
            Some(owner),
            LOGIN_PATH,
            Some(MANAGER),
            "GetSessionByPID",
            &(std::process::id(),),
        )
        .await;
    let reply = match by_pid {
        Ok(reply) => reply,
        Err(_) => {
            let id = id.filter(|id| !id.is_empty() && id.len() <= 128 && !id.contains('/'))?;
            bus.call_method(Some(owner), LOGIN_PATH, Some(MANAGER), "GetSession", &(id,))
                .await
                .ok()?
        }
    };
    let path: OwnedObjectPath = payload(&reply).ok()?;
    if !path
        .as_str()
        .starts_with("/org/freedesktop/login1/session/")
        || path.as_str().len() > 256
    {
        return None;
    }
    let (value, _) = property(bus, owner, path.as_str(), SESSION, "User")
        .await
        .ok()?;
    let (uid, _): (u32, OwnedObjectPath) = value.try_into().ok()?;
    (uid == unsafe { libc::geteuid() }).then(|| path.to_string())
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
