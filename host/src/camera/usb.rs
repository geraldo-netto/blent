//! Linux USB camera adapter. The listener and original portable owner survive
//! pipeline cancellation through independently observed native/remote cleanup.
use crate::{
    broker::linux::{client::Launch, presentation, registration_client},
    camera_registration,
    connection_discovery::Discovery,
    media_registration::{MediaRegistration, State},
};
use anyhow::{ensure, Result};
use blent_config::{
    camera::{
        connection::{Connection, Invitation, InvitationResult},
        CameraProfile,
    },
    connection::registration_client::{Client, Factory, Request},
    retirement::{Admission, Observe, Pool},
};
use std::{
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

struct Bound {
    factory: Arc<registration_client::Factory>,
    listener: TcpListener,
}
impl Factory for Bound {
    fn start(&mut self, request: Request) -> Result<Box<dyn Client>> {
        self.factory
            .start(request, vec![self.listener.try_clone()?])
            .map(|client| Box::new(client) as Box<dyn Client>)
    }
}

pub(crate) struct Bridge {
    owner: Mutex<Option<MediaRegistration>>,
    admission: Option<Admission>,
    profile: CameraProfile,
    token: String,
    invited: AtomicBool,
}
impl Bridge {
    pub(crate) fn native(
        profile: &CameraProfile,
        token: &str,
        listener: TcpListener,
        launch: Launch,
    ) -> Result<Self> {
        let source = Discovery::new(
            Box::new(presentation::Factory::new(presentation::Settings::new(
                launch.clone(),
            ))?),
            Duration::from_secs(1),
            Instant::now(),
        )?;
        let bound = Bound {
            factory: Arc::new(registration_client::Factory::new(
                registration_client::Settings::new(launch),
            )?),
            listener,
        };
        let owner = MediaRegistration::new(
            camera_registration::request(profile, token)?,
            source,
            Box::new(bound),
        )?;
        Self::owned(profile, token, owner)
    }
    fn owned(profile: &CameraProfile, token: &str, owner: MediaRegistration) -> Result<Self> {
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
        operation(
            owner
                .as_mut()
                .expect("camera bridge retains original owner"),
        )
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
    async fn invite_once(&self, token: &str, profile: &CameraProfile) -> Result<()> {
        ensure!(
            blent_config::runtime::token_matches(&self.token, token) && self.profile == *profile,
            "USB camera invitation differs from original request"
        );
        ensure!(
            !self.invited.swap(true, Ordering::SeqCst),
            "USB camera invitation already requested"
        );
        loop {
            match self.poll()? {
                State::Registering | State::Active => return Ok(()),
                State::Waiting => tokio::time::sleep(Duration::from_millis(100)).await,
                _ => anyhow::bail!("USB camera invitation closed"),
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
                "USB camera connection closed"
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
            "USB camera retirement incomplete; original ownership retained"
        );
        Ok(())
    }
}
impl Invitation for Bridge {
    fn invite<'a>(&'a self, token: &'a str, profile: &'a CameraProfile) -> InvitationResult<'a> {
        Box::pin(self.invite_once(token, profile))
    }
}
impl Connection for Bridge {
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
impl Drop for Bridge {
    fn drop(&mut self) {
        let mut owner = self
            .owner
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
            .expect("camera bridge drops original owner once");
        owner.close();
        self.admission
            .take()
            .expect("camera bridge retains original admission")
            .retain(Retained(owner));
    }
}

#[cfg(test)]
mod tests;
