//! Portable original USB media invitation supervision. Typed resource adapters
//! project requests; OS descriptor/process setup remains in the Linux boundary.
use crate::media_registration::{MediaRegistration, State};
use anyhow::{ensure, Result};
use blent_config::{
    connection::media::{Connection, Invitation, InvitationResult},
    retirement::{Admission, Observe, Pool},
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

pub(crate) struct Bridge<P> {
    owner: Mutex<Option<MediaRegistration>>,
    admission: Option<Admission>,
    profile: P,
    token: String,
    invited: AtomicBool,
}
impl<P: Clone + PartialEq + Send + Sync> Bridge<P> {
    pub(crate) fn owned(profile: &P, token: &str, owner: MediaRegistration) -> Result<Self> {
        let admission = Pool::new(1)?.reserve()?;
        Ok(Self {
            owner: Mutex::new(Some(owner)),
            admission: Some(admission),
            profile: profile.clone(),
            token: token.to_owned(),
            invited: AtomicBool::new(false),
        })
    }
    fn with_owner<T>(&self, operation: impl FnOnce(&mut MediaRegistration) -> T) -> T {
        let mut owner = self
            .owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        operation(owner.as_mut().expect("media bridge retains original owner"))
    }
    fn poll(&self) -> Result<State> {
        self.with_owner(|owner| {
            owner.poll(Instant::now())?;
            Ok(owner.state())
        })
    }
    pub(crate) fn retired(&self) -> bool {
        self.with_owner(|owner| owner.retired())
    }
    async fn invite_once(&self, token: &str, profile: &P) -> Result<()> {
        ensure!(
            blent_config::runtime::token_matches(&self.token, token) && self.profile == *profile,
            "USB media invitation differs from original request"
        );
        ensure!(
            !self.invited.swap(true, Ordering::SeqCst),
            "USB media invitation already requested"
        );
        loop {
            match self.poll()? {
                State::Registering | State::Active => return Ok(()),
                State::Waiting => tokio::time::sleep(Duration::from_millis(100)).await,
                _ => anyhow::bail!("USB media invitation closed"),
            }
        }
    }
    async fn supervise(&self) -> Result<()> {
        loop {
            ensure!(
                matches!(
                    self.poll()?,
                    State::Waiting | State::Registering | State::Active
                ),
                "USB media connection closed"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    async fn revoke(&self) -> Result<()> {
        self.with_owner(MediaRegistration::close);
        let result = tokio::time::timeout(Duration::from_secs(3), async {
            while !self.retired() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        ensure!(
            result.is_ok(),
            "USB media retirement incomplete; original ownership retained"
        );
        Ok(())
    }
}
impl<P: Clone + PartialEq + Send + Sync> Invitation<P> for Bridge<P> {
    fn invite<'a>(&'a self, token: &'a str, profile: &'a P) -> InvitationResult<'a> {
        Box::pin(self.invite_once(token, profile))
    }
}
impl<P: Clone + PartialEq + Send + Sync> Connection<P> for Bridge<P> {
    fn monitor(&self) -> InvitationResult<'_> {
        Box::pin(self.supervise())
    }
    fn close(&self) -> InvitationResult<'_> {
        Box::pin(self.revoke())
    }
}
struct Retained(MediaRegistration);
impl Observe for Retained {
    fn poll_complete(&mut self) -> bool {
        self.0.retired()
    }
}
impl<P> Drop for Bridge<P> {
    fn drop(&mut self) {
        let mut owner = self
            .owner
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
            .expect("media bridge drops original owner once");
        owner.close();
        self.admission
            .take()
            .expect("media bridge retains original admission")
            .retain(Retained(owner));
    }
}

#[cfg(all(target_os = "linux", feature = "accessory-transport"))]
pub(crate) mod linux;

#[cfg(test)]
mod tests;
