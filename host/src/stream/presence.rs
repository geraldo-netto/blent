//! Presence belongs to one admitted original video socket, never a replacement.
use anyhow::Result;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::watch,
};

pub(super) async fn watch(
    reader: &mut (impl AsyncRead + Unpin),
    _original: watch::Sender<bool>,
) -> Result<()> {
    let _ = reader.read_u8().await?;
    anyhow::bail!("unexpected input after video authentication")
}

#[derive(Clone)]
pub(super) struct View(watch::Receiver<bool>);
pub(super) fn channel() -> (watch::Sender<bool>, View) {
    let (publish, view) = watch::channel(true);
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

impl crate::capture_contract::LivenessCapability for View {
    fn supported(&self) -> bool {
        View::supported(self)
    }
    fn changed(&self) -> crate::capture_contract::Signal<'static> {
        let mut view = self.0.clone();
        let open = view.has_changed().is_ok();
        view.borrow_and_update();
        Box::pin(async move {
            if open {
                let _ = view.changed().await;
            } else {
                std::future::pending::<()>().await;
            }
        })
    }
}
