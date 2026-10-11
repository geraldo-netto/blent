//! Local cancellation is a best-effort relay, separate from strict wire and
//! native callbacks. A queued original request can outlive actual retirement;
//! it cannot revoke a replacement or turn an obsolete Stop into link failure.
use super::*;

pub(crate) fn cancel_original<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut Session<S>,
    lease: &Lease,
) -> Result<()> {
    let original = session.slots[lease.key.resource as usize - 1]
        .as_ref()
        .is_some_and(|binding| binding.lease == *lease);
    if original {
        session.cancel(lease, Reason::Requested)?;
    }
    Ok(())
}
