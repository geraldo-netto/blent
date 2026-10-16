//! One acknowledgement publisher per admitted original video socket.
use anyhow::Result;
use blent_config::video::negotiation;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::watch,
};

pub(super) async fn watch(
    reader: &mut (impl AsyncRead + Unpin),
    optional_token: bool,
    negotiated: watch::Sender<bool>,
) -> Result<()> {
    let first = reader.read_u8().await?;
    let first = prefix(reader, first, optional_token).await?;
    if first != negotiation::ACKNOWLEDGEMENT[0] {
        return Ok(());
    }
    let mut record = [0; 8];
    record[0] = first;
    tokio::time::timeout(super::AUTH_TIMEOUT, reader.read_exact(&mut record[1..])).await??;
    anyhow::ensure!(
        negotiation::acknowledged(&record),
        "invalid video liveness acknowledgement"
    );
    negotiated.send_replace(true);
    let _ = reader.read_u8().await?;
    Ok(())
}
async fn prefix(reader: &mut (impl AsyncRead + Unpin), first: u8, optional: bool) -> Result<u8> {
    if !optional || !first.is_ascii_hexdigit() {
        return Ok(first);
    }
    let mut rest = [0; 63];
    tokio::time::timeout(super::AUTH_TIMEOUT, reader.read_exact(&mut rest)).await??;
    anyhow::ensure!(
        rest.iter().all(u8::is_ascii_hexdigit),
        "invalid optional video token"
    );
    Ok(reader.read_u8().await?)
}

#[derive(Clone)]
pub(super) struct View(watch::Receiver<bool>);
pub(super) fn channel() -> (watch::Sender<bool>, View) {
    let (publish, view) = watch::channel(false);
    (publish, View(view))
}
impl View {
    pub(super) fn supported(&self) -> bool {
        self.0.has_changed().is_ok() && *self.0.borrow()
    }
    pub(super) async fn changed(&mut self) {
        if self.0.has_changed().is_err() {
            std::future::pending::<()>().await;
        }
        let _ = self.0.changed().await;
    }
}
