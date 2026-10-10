//! Registered media authority is independent of legacy ADB token preferences.
//! The composition owner supplies credentials only for its current broker route;
//! advertised discovery metadata alone must never call admitted().
use super::{Attachment, Authentication, Identity, Lease, Link, State};
use anyhow::{ensure, Result};
use blent_config::connection::{management::Credential, pairing::PeerId};

pub struct RegisteredRoute {
    lease: Lease,
}
impl Lease {
    /// Atomically renew only this observed generation. No check-then-register
    /// window can overwrite a competing attachment, including during retry.
    pub fn register_route(
        &self,
        peer: PeerId,
        credential: Credential,
        link: Link,
    ) -> Result<RegisteredRoute> {
        self.attachment
            .register_guarded(peer, credential, link, Some(self.generation))
    }
    /// Detach only this current native generation. A revoked lease may retain
    /// this authority through hot-renewal backoff; stale or legacy leases cannot
    /// remove a replacement. No token or native retirement proof is created.
    pub fn detach_registered(&self) -> bool {
        let attachment = &self.attachment;
        let mut state = attachment
            .0
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed || state.generation != self.generation || !state.registered {
            return false;
        }
        state.authentication = Authentication::Unavailable;
        state.identity = None;
        state.transport = None;
        attachment.reset_metadata();
        attachment.0.presence.send_replace(false);
        let _ = attachment.advance_registered(&mut state);
        true
    }
}
impl Attachment {
    /// Prepare credentials before registration can connect back to the owned
    /// listeners. Same-peer renewal keeps configured geometry and display presence;
    /// old socket authority is revoked synchronously regardless of that continuity.
    pub fn register_route(
        &self,
        peer: PeerId,
        credential: Credential,
        link: Link,
    ) -> Result<RegisteredRoute> {
        self.register_guarded(peer, credential, link, None)
    }
    fn register_guarded(
        &self,
        peer: PeerId,
        credential: Credential,
        link: Link,
        expected: Option<u64>,
    ) -> Result<RegisteredRoute> {
        let identity = Identity::authenticated(peer);
        let mut state = self.0.state.lock().unwrap();
        ensure!(!state.closed, "session attachment retired");
        ensure!(
            expected.is_none_or(|expected| expected == state.generation),
            "stale display owner"
        );
        let preserve = state.identity.as_ref() == Some(&identity);
        self.advance_registered(&mut state)?;
        state.authentication = Authentication::Required(token(&credential));
        state.registered = true;
        state.identity = Some(identity);
        state.transport = Some(link);
        if !preserve {
            self.reset_metadata();
            self.0.presence.send_replace(false);
        }
        Ok(RegisteredRoute {
            lease: self.lease_locked(&state),
        })
    }
    pub(super) fn advance_registered(&self, state: &mut State) -> Result<()> {
        match state.generation.checked_add(1) {
            Some(next) => state.generation = next,
            None => {
                state.closed = true;
                state.authentication = Authentication::Unavailable;
                self.0.presence.send_replace(false);
                self.reset_metadata();
            }
        }
        // send_replace wakes outstanding retirement watchers even at exhaustion.
        self.0.generation.send_replace(state.generation);
        ensure!(!state.closed, "attachment generation exhausted");
        Ok(())
    }
}
impl State {
    pub(super) fn observed_authentication(&mut self, identity: &Option<Identity>, preserve: bool) {
        if self.registered {
            self.authentication = Authentication::Unavailable;
            // Absence and descriptive authenticated metadata cannot restore
            // tokenless access while old native routes are still retiring.
            if identity
                .as_ref()
                .is_some_and(|identity| identity.peer().is_none())
            {
                self.authentication = self.legacy.clone();
                self.registered = false;
            }
        }
        if !self.registered {
            self.authentication
                .rotate(preserve, blent_config::credentials::random_token);
        }
    }
}
impl RegisteredRoute {
    /// Exact current broker Active proof is required by the caller. Logical
    /// admission does not substitute for remote/native retirement ownership.
    pub fn admitted(&self) -> bool {
        self.lease.apply(|| {
            self.lease.attachment.0.presence.send_replace(true);
        })
    }
    pub fn current(&self) -> bool {
        self.lease.apply(|| ())
    }
    /// Revoke old sockets while retaining same-peer display intent. The returned
    /// guard can later detach this exact revoked generation on whole-peer loss.
    /// Capture that guard under the same lock; never snapshot a competing route.
    pub fn revoke(&self) -> Option<Lease> {
        let attachment = &self.lease.attachment;
        let mut state = attachment
            .0
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed || state.generation != self.lease.generation {
            return None;
        }
        state.authentication = Authentication::Unavailable;
        attachment.advance_registered(&mut state).ok()?;
        Some(attachment.lease_locked(&state))
    }
    pub fn close(&self) {
        let _ = self.revoke();
    }
}
impl Drop for RegisteredRoute {
    fn drop(&mut self) {
        self.close();
    }
}
fn token(credential: &Credential) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in credential.as_bytes() {
        out.push(HEX[usize::from(byte >> 4)] as char);
        out.push(HEX[usize::from(byte & 15)] as char);
    }
    out
}

#[cfg(test)]
mod tests;
