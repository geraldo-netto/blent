//! A failed accept funds one retry deadline; successful/idle accepts need no timer.
//! The server owns this scalar state. Each call borrows its listener; cancellation
//! preserves the deadline and admits no socket (Tokio accept is cancellation safe).
use std::{io, net::SocketAddr, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    time::Instant,
};

#[derive(Default)]
pub(crate) struct Accept {
    retry: Option<Instant>,
}
impl Accept {
    pub(crate) async fn next(
        &mut self,
        listener: &TcpListener,
    ) -> io::Result<(TcpStream, SocketAddr)> {
        if let Some(deadline) = self.retry {
            tokio::time::sleep_until(deadline).await;
        }
        let result = listener.accept().await;
        self.retry = result
            .as_ref()
            .err()
            .map(|_| Instant::now() + Duration::from_millis(100));
        result
    }
}
