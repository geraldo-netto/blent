//! Descriptive display assignment status, independent of native APIs. A live
//! daemon report is not TLS proof or permission to open a resource.
use crate::connection::pairing::PeerId;
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Identity {
    Adb(String),
    UsbPeer(PeerId),
}
impl Identity {
    pub fn adb_serial(&self) -> Option<&str> {
        match self {
            Self::Adb(serial) => Some(serial),
            Self::UsbPeer(_) => None,
        }
    }
    pub fn peer(&self) -> Option<&PeerId> {
        match self {
            Self::UsbPeer(peer) => Some(peer),
            Self::Adb(_) => None,
        }
    }
    pub fn label(&self) -> String {
        match self {
            Self::Adb(serial) => serial.clone(),
            Self::UsbPeer(peer) => format!("USB tablet ({})", &peer.as_str()[..12]),
        }
    }
    fn validate(&self) -> Result<()> {
        if let Self::Adb(serial) = self {
            ensure!(
                !serial.is_empty() && serial.len() <= 1024 && !serial.chars().any(char::is_control),
                "invalid ADB identity"
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabletSession {
    pub identity: Identity,
    pub instance: u32,
    pub video_port: u16,
    pub input_port: u16,
}
impl TabletSession {
    fn validate(&self) -> Result<()> {
        self.identity.validate()?;
        ensure!(self.instance < 4, "invalid display instance");
        ensure!(
            self.video_port != 0 && self.input_port != 0 && self.video_port != self.input_port,
            "invalid display ports"
        );
        Ok(())
    }
}
// Existing ADB JSON keeps its exact field shape. Native records have peer
// instead of serial; never encode a certificate as a made-up ADB device alias.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    #[serde(default)]
    serial: Option<String>,
    #[serde(default)]
    peer: Option<String>,
    instance: u32,
    video_port: u16,
    input_port: u16,
}
#[derive(Serialize)]
struct Output<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    serial: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    peer: Option<&'a str>,
    instance: u32,
    video_port: u16,
    input_port: u16,
}
impl TryFrom<Record> for TabletSession {
    type Error = anyhow::Error;
    fn try_from(record: Record) -> Result<Self> {
        let identity = match (record.serial, record.peer) {
            (Some(serial), None) => Identity::Adb(serial),
            (None, Some(peer)) => Identity::UsbPeer(PeerId::parse(peer)?),
            _ => anyhow::bail!("ambiguous or missing display identity"),
        };
        let session = Self {
            identity,
            instance: record.instance,
            video_port: record.video_port,
            input_port: record.input_port,
        };
        session.validate()?;
        Ok(session)
    }
}
impl Serialize for TabletSession {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.validate().map_err(serde::ser::Error::custom)?;
        Output {
            serial: self.identity.adb_serial(),
            peer: self.identity.peer().map(PeerId::as_str),
            instance: self.instance,
            video_port: self.video_port,
            input_port: self.input_port,
        }
        .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for TabletSession {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        Self::try_from(Record::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
#[cfg(test)]
mod tests;
