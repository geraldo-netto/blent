//! Connection I/O and retirement boundary. Native adapters own device APIs and
//! process handles; TLS/approval and channel routing depend only on these traits.
use blent_config::retirement::Completion;
use std::{future::Future, pin::Pin};
use tokio::io::{AsyncRead, AsyncWrite};

pub trait Stream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Stream for T {}
pub trait Owner: Send {
    /// True only after actual handle/worker retirement. A timeout never suffices.
    fn completion(&self) -> Completion;
    fn retire(self: Box<Self>) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>;
}
pub struct Resource {
    pub stream: Box<dyn Stream>,
    pub owner: Box<dyn Owner>,
}

#[cfg(target_os = "linux")]
pub mod linux;

pub mod setup;
