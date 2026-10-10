//! Registered media authority is independent of legacy ADB token preferences.
//! The composition owner supplies credentials only for its current broker route;
//! advertised discovery metadata alone must never call admitted().
use super::{Attachment, Authentication, Identity, Lease, Link, State};
use anyhow::{ensure, Result};
use blent_config::connection::{management::Credential, pairing::PeerId};

pub struct RegisteredRoute {
    lease: Lease,
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
        let identity = Identity::authenticated(peer);
        let mut state = self.0.state.lock().unwrap();
        ensure!(!state.closed, "session attachment retired");
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
    /// Revoke old sockets immediately while retaining same-peer display intent.
    /// Whole-peer loss must also clear Attachment presence via its owner.
    pub fn close(&self) {
        let attachment = &self.lease.attachment;
        let mut state = attachment
            .0
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed || state.generation != self.lease.generation {
            return;
        }
        state.authentication = Authentication::Unavailable;
        let _ = attachment.advance_registered(&mut state);
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
