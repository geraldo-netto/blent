//! Temporary endpoint retirement policy, independent of ADB processes and OS IO.
use anyhow::{ensure, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Missing,
    Owned,
    Foreign,
}

pub fn route(text: &str, remote: &str, local: &str) -> Result<Route> {
    let mappings = super::parse(text)?;
    Ok(match mappings.get(remote) {
        None => Route::Missing,
        Some(target) if *target == local => Route::Owned,
        Some(_) => Route::Foreign,
    })
}

pub fn port(endpoint: &str) -> Result<u16> {
    let port = endpoint.strip_prefix("tcp:").unwrap_or("").parse::<u16>()?;
    ensure!(port != 0, "ADB endpoint requires a nonzero TCP port");
    Ok(port)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Creation {
    NotStarted,
    Uncertain,
    Acknowledged,
    RemovalPending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cleanup {
    Complete,
    Remove,
    Wait,
}

impl Creation {
    /// An unacknowledged creator may still publish its single delegated effect.
    /// Absence alone is not a fence. Observing its exclusively owned endpoint
    /// confirms that effect occurred; the native adapter must also join commands.
    pub fn cleanup(self, route: Route) -> Cleanup {
        match (self, route) {
            (Self::NotStarted, _) => Cleanup::Complete,
            (Self::RemovalPending, Route::Missing) => Cleanup::Complete,
            (Self::RemovalPending, _) => Cleanup::Wait,
            (_, Route::Owned) => Cleanup::Remove,
            (Self::Acknowledged, _) => Cleanup::Complete,
            _ => Cleanup::Wait,
        }
    }
}

#[cfg(test)]
mod tests;
