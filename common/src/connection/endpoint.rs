//! Nonblocking media-channel boundary for local byte-stream adapters.
//! A single owner serializes these calls with connection/resource revocation.
use anyhow::Result;

/// Exact outstanding consumer borrow; equal byte counts are not interchangeable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Read {
    pub id: u64,
    pub count: usize,
}

pub trait Endpoint {
    /// Maximum retained allocation accepted by the outbound queue. Constant
    /// for this endpoint lease; payload length alone does not bound allocation.
    fn capacity(&self) -> usize;
    fn identity(&self) -> (&super::resources::Lease, super::wire::Channel);
    /// Full current connection/resource/channel authority, not cached readiness.
    fn active(&self) -> bool;
    /// On backpressure return the same allocation, without consuming bytes.
    fn offer(&mut self, bytes: Vec<u8>) -> std::result::Result<(), Vec<u8>>;
    /// At most one outstanding borrow. Credit remains charged until consumed.
    fn read(&mut self, out: &mut [u8]) -> Result<Option<Read>>;
    /// Caller has completed the entire local write and flush for this borrow.
    fn consumed(&mut self, read: Read) -> Result<()>;
}
