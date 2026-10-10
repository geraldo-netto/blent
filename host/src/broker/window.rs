//! T875 bounded negotiation after mutual TLS, before constructing Session.
use super::session::Limits;
use anyhow::{ensure, Result};
use blent_config::connection::window;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Caller supplies an authenticated stream and rechecks its pairing lease after
/// this await. Any error or cancellation makes the setup stream unusable: drop
/// it and observe native retirement before reusing its admission slot.
pub async fn negotiate<S: AsyncRead + AsyncWrite + Unpin + ?Sized>(
    stream: &mut S,
    mut limits: Limits,
) -> Result<Limits> {
    window::validate(limits.channel_bytes)?;
    ensure!(
        !limits.io_timeout.is_zero() && limits.io_timeout <= Duration::from_secs(60),
        "invalid window exchange deadline"
    );
    limits.channel_bytes =
        tokio::time::timeout(limits.io_timeout, exchange(stream, limits.channel_bytes)).await??;
    Ok(limits)
}

async fn exchange<S: AsyncRead + AsyncWrite + Unpin + ?Sized>(
    stream: &mut S,
    proposed: u32,
) -> Result<u32> {
    stream.write_all(&window::encode(true, proposed)?).await?;
    stream.flush().await?;
    let mut bytes = [0; window::BYTES];
    stream.read_exact(&mut bytes).await?;
    window::accepted(proposed, window::decode(&bytes, false)?)
}

#[cfg(test)]
mod tests;
