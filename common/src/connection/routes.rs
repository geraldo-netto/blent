//! T856: portable peer metadata and exclusive local loopback-route ownership.
//! A route claim is local authority only, never proof of TLS or media consent.
use super::{pairing::PeerId, wire::MAX_COUNTER};
use anyhow::{ensure, Result};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    Usb,
    Network,
}
impl Link {
    pub fn profile_name(self) -> &'static str {
        match self {
            Self::Usb => "usb",
            Self::Network => "network",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum IdentityKey {
    Observed(String),
    Authenticated(PeerId),
}
/// Adapters supply continuity and optional saved-profile identity explicitly.
/// Domain code never parses transport-specific prefixes to infer either one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    key: IdentityKey,
    profile: Option<String>,
}
impl Identity {
    pub fn observed(key: String, profile: Option<String>) -> Result<Self> {
        validate_identity(&key)?;
        if let Some(profile) = &profile {
            validate_identity(profile)?;
        }
        Ok(Self {
            key: IdentityKey::Observed(key),
            profile,
        })
    }
    /// Called only after the connection adapter proves the exact certificate.
    /// Native profiles have their own identity; no guessed ADB/device alias.
    pub fn authenticated(peer: PeerId) -> Self {
        let profile = Some(format!("certificate:{}", peer.as_str()));
        Self {
            key: IdentityKey::Authenticated(peer),
            profile,
        }
    }
    pub fn peer(&self) -> Option<&PeerId> {
        match &self.key {
            IdentityKey::Authenticated(peer) => Some(peer),
            _ => None,
        }
    }
    pub fn profile(&self) -> Option<&str> {
        self.profile.as_deref()
    }
}
fn validate_identity(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control),
        "invalid local identity"
    );
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteId {
    pub owner: u64,
    pub revision: u64,
}
struct Claim {
    id: RouteId,
    ports: BTreeSet<u16>,
    closing: bool,
}
/// One registry per local network namespace. Owners are authenticated local
/// registration IDs, not remote-supplied process IDs. Ports are assigned by the
/// local OS adapter; no hostname, path or remote address is representable.
/// Metadata is bounded by configured capacity and five ports per registration.
/// BTreeMap lookup avoids a scan across every attached peer/resource.
pub struct RouteClaims {
    capacity: usize,
    revision: u64,
    owners: BTreeMap<u64, Claim>,
    ports: BTreeMap<u16, RouteId>,
}
impl RouteClaims {
    pub fn new(capacity: usize) -> Result<Self> {
        ensure!(
            (1..=1024).contains(&capacity),
            "invalid local route capacity"
        );
        Ok(Self {
            capacity,
            revision: 0,
            owners: BTreeMap::new(),
            ports: BTreeMap::new(),
        })
    }
    /// Reserve before publishing endpoints. Failed admission changes nothing.
    /// The OS listener adapter must still bind the exact ports exclusively.
    pub fn reserve(&mut self, owner: u64, ports: &[u32]) -> Result<RouteId> {
        ensure!(
            (1..=MAX_COUNTER).contains(&owner),
            "invalid local route owner"
        );
        ensure!(
            self.revision < MAX_COUNTER && self.owners.len() < self.capacity,
            "local route capacity exhausted"
        );
        ensure!(
            !self.owners.contains_key(&owner),
            "local owner still registered"
        );
        let ports = self.validate_ports(ports)?;
        let id = RouteId {
            owner,
            revision: self.revision + 1,
        };
        for port in &ports {
            self.ports.insert(*port, id);
        }
        self.owners.insert(
            owner,
            Claim {
                id,
                ports,
                closing: false,
            },
        );
        self.revision = id.revision;
        Ok(id)
    }
    fn validate_ports(&self, ports: &[u32]) -> Result<BTreeSet<u16>> {
        ensure!((1..=5).contains(&ports.len()), "invalid local port count");
        let mut unique = BTreeSet::new();
        for port in ports {
            ensure!((1..=65535).contains(port), "invalid local port");
            let port = *port as u16;
            ensure!(
                !self.ports.contains_key(&port) && unique.insert(port),
                "local port already claimed"
            );
        }
        Ok(unique)
    }
    pub fn current(&self, id: RouteId, port: u32) -> bool {
        let Ok(port) = u16::try_from(port) else {
            return false;
        };
        self.owners
            .get(&id.owner)
            .is_some_and(|claim| claim.id == id && !claim.closing && claim.ports.contains(&port))
    }
    fn claim(&mut self, id: RouteId) -> Result<&mut Claim> {
        let claim = self
            .owners
            .get_mut(&id.owner)
            .ok_or_else(|| anyhow::anyhow!("local owner absent"))?;
        ensure!(claim.id == id, "stale local route");
        Ok(claim)
    }
    pub fn revoke(&mut self, id: RouteId) -> Result<bool> {
        let claim = self.claim(id)?;
        let changed = !claim.closing;
        claim.closing = true;
        Ok(changed)
    }
    /// Only actual listener/bridge retirement releases admission. Cancellation,
    /// process kill requests and user-visible timeouts leave the claim occupied.
    pub fn retired(&mut self, id: RouteId) -> Result<()> {
        ensure!(self.claim(id)?.closing, "local route not revoked");
        let claim = self.owners.remove(&id.owner).expect("validated claim");
        for port in claim.ports {
            self.ports.remove(&port);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t856_local_route_revision_exhaustion_does_not_wrap() {
        let mut routes = RouteClaims::new(2).unwrap();
        routes.revision = MAX_COUNTER - 1;
        let last = routes.reserve(1, &[12345]).unwrap();
        assert_eq!(last.revision, MAX_COUNTER);
        routes.revoke(last).unwrap();
        routes.retired(last).unwrap();
        assert!(routes.reserve(1, &[12345]).is_err());
        assert!(routes.reserve(2, &[12346]).is_err());
        assert!(!routes.current(last, 12345));
    }
}
