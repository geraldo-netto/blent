//! Bounded policy exceptions, owned by Slots for one daemon lifetime. Manual USB
//! Start is cooperative; original automatic discovery retains its priority rule.
use super::Owner;
use anyhow::{ensure, Result};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Policy {
    Suppressed,
    Cooperative,
}
struct Entry {
    owner: Owner,
    policy: Policy,
}
#[derive(Default)]
pub(super) struct Intents {
    entries: Vec<Entry>,
}

impl Intents {
    fn policy(&self, owner: &Owner) -> Option<Policy> {
        self.entries
            .iter()
            .find(|entry| &entry.owner == owner)
            .map(|entry| entry.policy)
    }
    pub(super) fn enabled(&self, owner: &Owner) -> bool {
        self.policy(owner) != Some(Policy::Suppressed)
    }
    pub(super) fn preempts(&self, owner: &Owner) -> bool {
        self.policy(owner) != Some(Policy::Cooperative)
    }

    pub(super) fn set_enabled(&mut self, owner: Owner, enabled: bool) -> Result<()> {
        owner.target().identity()?;
        let policy = requested(&owner, enabled);
        if let Some(index) = self.entries.iter().position(|entry| entry.owner == owner) {
            match policy {
                Some(policy) => self.entries[index].policy = policy,
                None => {
                    self.entries.swap_remove(index);
                }
            }
        } else if let Some(policy) = policy {
            ensure!(
                self.entries.len() < 1024,
                "display intent capacity exceeded"
            );
            self.entries.push(Entry { owner, policy });
        }
        Ok(())
    }
}

fn requested(owner: &Owner, enabled: bool) -> Option<Policy> {
    if !enabled {
        return Some(Policy::Suppressed);
    }
    match owner {
        Owner::Peer(_) => Some(Policy::Cooperative),
        Owner::Adb(_) => None,
    }
}
