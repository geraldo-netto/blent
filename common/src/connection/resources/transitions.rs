use super::*;
impl Resources {
    /// Tablet commits Prepared to bounded FIFO output only after local consent
    /// and preparation. Host calls this on receipt. Neither grants media access.
    pub fn prepared(&mut self, lease: &Lease, now: i64) -> Result<bool> {
        let role = self.role;
        let slot = self.online(lease)?;
        ensure!(
            slot.stage == Stage::Offered && !slot.rejected,
            "unexpected Prepared"
        );
        if role == Role::Tablet {
            slot.live(now)?;
        }
        ensure!(now >= 0, "invalid monotonic time");
        slot.closing |= now >= slot.deadline;
        slot.stage = Stage::Prepared;
        Ok(!slot.closing)
    }
    pub fn permission_required(&mut self, lease: &Lease, now: i64) -> Result<bool> {
        let role = self.role;
        let slot = self.online(lease)?;
        ensure!(
            slot.stage == Stage::Offered && !slot.permission_reported && !slot.rejected,
            "unexpected permission result"
        );
        if role == Role::Tablet {
            slot.live(now)?;
        }
        ensure!(now >= 0, "invalid monotonic time");
        slot.permission_reported = true;
        slot.closing |= now >= slot.deadline;
        Ok(!slot.closing)
    }
    /// Pre-Prepared failures only. Later failure uses Revoke. Revoked is
    /// reserved exclusively for the terminal acknowledgement, never rejection.
    pub fn reject(&mut self, lease: &Lease, outcome: Outcome) -> Result<()> {
        ensure!(
            matches!(
                outcome,
                Outcome::Unavailable | Outcome::PermissionDenied | Outcome::Busy | Outcome::Failed
            ),
            "not a rejection"
        );
        let role = self.role;
        let slot = self.online(lease)?;
        ensure!(
            slot.stage == Stage::Offered && !slot.rejected,
            "unexpected rejection"
        );
        ensure!(role == Role::Host || !slot.closing, "local result revoked");
        slot.rejected = true;
        slot.closing = true;
        Ok(())
    }
    /// Reserve output before this call. Cancellation cannot remove/reorder a
    /// started Commit; process its whole-record completion before host Revoke.
    pub fn start_commit(&mut self, lease: &Lease, now: i64) -> Result<()> {
        ensure!(self.role == Role::Host, "only host commits");
        let slot = self.online(lease)?;
        slot.live(now)?;
        ensure!(slot.stage == Stage::Prepared, "resource not prepared");
        slot.stage = Stage::Committing;
        Ok(())
    }
    /// Host: entire Commit record written, borrow retired, before peer events.
    /// Tablet: complete Commit received, before next envelope. Install Drain
    /// closes the agreed inactive generation without granting credit/authority.
    pub fn commit(&mut self, lease: &Lease, now: i64) -> Result<Install> {
        let expected = if self.role == Role::Host {
            Stage::Committing
        } else {
            Stage::Prepared
        };
        let slot = self.online(lease)?;
        ensure!(
            now >= 0 && slot.stage == expected && !slot.peer_revoke_for_tablet(expected),
            "unexpected Commit"
        );
        slot.closing |= now >= slot.deadline;
        slot.stage = Stage::Committed;
        slot.installed = true;
        slot.channels_retired = false;
        Ok(if slot.closing {
            Install::Drain
        } else {
            Install::Activate
        })
    }
    /// Tablet commits Result(Admitted) to FIFO after preparation; host consumes
    /// it. This grants a lease, not proof that a media stream actually works.
    pub fn admitted(&mut self, lease: &Lease, now: i64) -> Result<bool> {
        let role = self.role;
        let slot = self.online(lease)?;
        ensure!(slot.stage == Stage::Committed, "unexpected admission");
        if role == Role::Tablet {
            slot.live(now)?;
        }
        ensure!(now >= 0, "invalid monotonic time");
        slot.closing |= now >= slot.deadline;
        slot.stage = Stage::Admitted;
        Ok(!slot.closing)
    }
    pub fn peer_revoke(&mut self, lease: &Lease) -> Result<()> {
        let slot = self.online(lease)?;
        ensure!(!slot.peer_revoke, "duplicate peer revoke");
        slot.peer_revoke = true;
        slot.closing = true;
        Ok(())
    }
    /// Host's terminal coordination record, ordered after any Commit. Tablet
    /// replies only after its own actual retirement, never merely kill request.
    pub fn revoke_written(&mut self, lease: &Lease) -> Result<()> {
        ensure!(self.role == Role::Host, "only host coordinates settlement");
        let slot = self.online(lease)?;
        ensure!(
            slot.closing && slot.stage != Stage::Committing && !slot.revoke_written,
            "revoke barrier unavailable"
        );
        slot.revoke_written = true;
        Ok(())
    }
    /// Tablet: call after whole Result(Revoked) record finishes. Host: call on
    /// receipt, behind revoke_written. Tablet must check ready_to_ack BEFORE
    /// queuing the ACK; this method checks again before releasing the tombstone.
    pub fn acknowledge(&mut self, lease: &Lease) -> Result<()> {
        ensure!(self.ready_to_ack(lease)?, "settlement not ready");
        self.online(lease)?.settled = true;
        Ok(())
    }
    pub fn ready_to_ack(&mut self, lease: &Lease) -> Result<bool> {
        let role = self.role;
        let slot = self.online(lease)?;
        if !slot.closing {
            return Ok(false);
        }
        Ok(match role {
            Role::Host => slot.revoke_written,
            Role::Tablet => slot.peer_revoke && slot.native_retired && slot.channels_retired,
        })
    }
}
impl Slot {
    fn peer_revoke_for_tablet(&self, expected: Stage) -> bool {
        expected == Stage::Prepared && self.peer_revoke
    }
}
