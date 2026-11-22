//! Independent bounded local IPC reader/writer; slow UI never owns media polling.
use super::{async_io::AsyncSocket, Received};
use crate::owned_task::Task;
use anyhow::{ensure, Context, Result};
use blent_config::{
    connection::registration::Message,
    retirement::{Admission, Completion},
};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;

const MAX_BATCH: usize = 1602; // 512 devices + 64 links + 1024 peers + Begin/End.
pub struct Peer {
    replies: mpsc::Sender<Vec<Message>>,
    requests: mpsc::Receiver<Received>,
    task: Task,
}
impl Peer {
    pub(crate) fn start(
        admission: Admission,
        socket: AsyncSocket,
        timeout: Duration,
    ) -> Result<Self> {
        ensure!(
            !timeout.is_zero() && timeout <= Duration::from_secs(60),
            "invalid local client deadline"
        );
        let (replies, outbound) = mpsc::channel(2);
        let (inbound, requests) = mpsc::channel(2);
        let socket = Arc::new(socket);
        let task = Task::spawn_reserved(admission, async move {
            tokio::try_join!(read(&socket, inbound), write(&socket, outbound, timeout))?;
            Ok(())
        });
        Ok(Self {
            replies,
            requests,
            task,
        })
    }
    pub fn reply(&self, messages: Vec<Message>) -> Result<()> {
        ensure!(
            (1..=MAX_BATCH).contains(&messages.len()),
            "invalid local reply batch"
        );
        self.replies
            .try_send(messages)
            .map_err(|_| anyhow::anyhow!("local client reply capacity exhausted"))
    }
    pub fn stop(&self) {
        self.task.abort();
    }
    pub fn completion(&self) -> Completion {
        self.task.completion()
    }
    pub fn poll_next(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<Received>> {
        self.task.poll_notice(&mut self.requests, cx)
    }
}
async fn read(socket: &AsyncSocket, inbound: mpsc::Sender<Received>) -> Result<()> {
    loop {
        let packet = socket.receive().await?;
        inbound
            .send(packet)
            .await
            .context("local client dispatcher closed")?;
    }
}
async fn write(
    socket: &AsyncSocket,
    mut outbound: mpsc::Receiver<Vec<Message>>,
    timeout: Duration,
) -> Result<()> {
    while let Some(batch) = outbound.recv().await {
        tokio::time::timeout(timeout, send_batch(socket, batch)).await??;
    }
    Ok(())
}
async fn send_batch(socket: &AsyncSocket, batch: Vec<Message>) -> Result<()> {
    for message in batch {
        socket.send(&message, &[]).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
