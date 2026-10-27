//! Descriptive tablet/resource views. No observation authorizes native effects,
//! grants consent, or proves cleanup. Adapters revalidate every actual action.
use crate::tablets::Identity;
use anyhow::{ensure, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Presence {
    Unknown,
    Available,
    Connected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    identity: Identity,
    name: String,
    presence: Presence,
    display_assigned: bool,
}

impl Device {
    pub fn new(
        identity: Identity,
        name: String,
        presence: Presence,
        display_assigned: bool,
    ) -> Result<Self> {
        validate_identity(&identity)?;
        ensure!(
            name.len() <= 1024 && !name.chars().any(char::is_control),
            "invalid tablet name"
        );
        let name = if name.is_empty() {
            identity.label()
        } else {
            name
        };
        Ok(Self {
            identity,
            name,
            presence,
            display_assigned,
        })
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn presence(&self) -> Presence {
        self.presence
    }
    pub fn display_assigned(&self) -> bool {
        self.display_assigned
    }

    fn merge(&mut self, other: Self) {
        self.display_assigned |= other.display_assigned;
        if (other.presence, std::cmp::Reverse(&other.name))
            > (self.presence, std::cmp::Reverse(&self.name))
        {
            self.name = other.name;
        }
        self.presence = self.presence.max(other.presence);
    }
}

pub(crate) fn validate_identity(identity: &Identity) -> Result<()> {
    if let Identity::Adb(serial) = identity {
        ensure!(
            !serial.is_empty() && serial.len() <= 1024 && !serial.chars().any(char::is_control),
            "invalid tablet identity"
        );
    }
    Ok(())
}

/// One bounded projection assembled from current source views. This is not a
/// persistent connection registry. Rebuild it when source evidence changes.
pub struct Catalog {
    devices: Vec<Device>,
    limit: usize,
}

impl Catalog {
    pub fn new(limit: usize) -> Result<Self> {
        ensure!((1..=1024).contains(&limit), "invalid workspace capacity");
        Ok(Self {
            devices: Vec::new(),
            limit,
        })
    }

    pub fn insert(&mut self, device: Device) -> Result<()> {
        if let Some(current) = self
            .devices
            .iter_mut()
            .find(|current| current.identity == device.identity)
        {
            current.merge(device);
            return Ok(());
        }
        ensure!(
            self.devices.len() < self.limit,
            "workspace capacity exhausted"
        );
        self.devices.push(device);
        Ok(())
    }

    pub fn devices(&self) -> &[Device] {
        &self.devices
    }
    pub fn into_devices(self) -> Vec<Device> {
        self.devices
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resource {
    Display,
    Camera,
    Microphone,
    Speakers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Unknown,
    Off,
    Starting,
    Waiting,
    InUse,
    Paused,
    Stopping,
    Failed,
}

impl Activity {
    pub fn is_active(self) -> bool {
        !matches!(self, Self::Off | Self::Failed)
    }
}

/// Borrowed descriptive metadata from the existing resource owner. An absent
/// target means unresolved/automatic selection, never the selected GUI tablet.
pub struct Observation<'a> {
    pub target: Option<&'a Identity>,
    pub activity: Activity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceView {
    Unavailable,
    NotStarted,
    Here(Activity),
    Elsewhere(Activity),
}

impl ResourceView {
    /// UI affordance only. The actual owner must validate admission and cleanup.
    pub fn can_start(self) -> bool {
        matches!(
            self,
            Self::NotStarted | Self::Here(Activity::Off | Activity::Failed)
        )
    }

    pub fn can_stop(self) -> bool {
        matches!(self, Self::Here(activity) if activity.is_active())
    }
}

pub fn project(
    selected: &Identity,
    supported: bool,
    owner: Option<Observation<'_>>,
) -> ResourceView {
    if let Some(owner) = owner {
        if owner.activity.is_active() {
            return if owner.target == Some(selected) {
                ResourceView::Here(owner.activity)
            } else {
                ResourceView::Elsewhere(owner.activity)
            };
        }
        if supported {
            return ResourceView::Here(owner.activity);
        }
    }
    if supported {
        ResourceView::NotStarted
    } else {
        ResourceView::Unavailable
    }
}

#[cfg(test)]
mod tests;
