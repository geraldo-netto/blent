//! One rejection exchange owned and polled by the server, never a detached task.
use super::super::async_io::AsyncSocket;
use blent_config::connection::registration::{Message, Rejection};
use std::{future::Future, pin::Pin, time::Duration};

#[derive(Default)]
pub(super) struct Rejector {
    pending: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
}
impl Rejector {
    pub fn active(&self) -> bool {
        self.pending.is_some()
    }
    pub fn start(&mut self, socket: AsyncSocket, timeout: Duration) {
        assert!(!self.active(), "only one overload exchange may be admitted");
        self.pending = Some(Box::pin(async move {
            let _ = tokio::time::timeout(timeout, reject(socket)).await;
        }));
    }
    pub async fn next(&mut self) {
        match self.pending.as_mut() {
            Some(pending) => pending.await,
            None => std::future::pending().await,
        }
        self.stop();
    }
    pub fn stop(&mut self) {
        // Dropping the original future closes its socket and received descriptors.
        // AsyncSocket has no detached syscall or callback borrowing these owners.
        self.pending.take();
    }
}
async fn reject(socket: AsyncSocket) -> std::io::Result<()> {
    let received = socket.receive().await?;
    let request = received.message.request();
    drop(received);
    socket
        .send(
            &Message::Rejected {
                request,
                reason: Rejection::Busy,
            },
            &[],
        )
        .await
}
