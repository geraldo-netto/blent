use super::*;
use crate::broker::linux::async_io::AsyncSocket;
use blent_config::connection::{
    control::Command,
    control_view::Snapshot,
    registration::{Message, Rejection},
    wire::MAX_COUNTER,
};

pub(super) enum Reply {
    Snapshot(Snapshot),
    Rejected(Rejection),
}
pub(super) fn next_request(previous: &mut u64) -> Result<u64> {
    ensure!(
        *previous < MAX_COUNTER,
        "broker client request counter exhausted"
    );
    *previous += 1;
    Ok(*previous)
}
pub(super) async fn exchange(
    socket: &AsyncSocket,
    request: u64,
    command: Command,
    limits: Limits,
    deadline: Duration,
) -> Result<Reply> {
    tokio::time::timeout(deadline, transaction(socket, request, command, limits)).await?
}
async fn transaction(
    socket: &AsyncSocket,
    request: u64,
    command: Command,
    limits: Limits,
) -> Result<Reply> {
    let mut assembler = Assembler::new(limits)?;
    socket
        .send(&Message::Control { request, command }, &[])
        .await?;
    loop {
        let received = socket.receive().await?;
        ensure!(
            received.descriptors.is_empty(),
            "unexpected descriptor in broker control reply"
        );
        ensure!(
            received.message.request() == request,
            "broker control reply request mismatch"
        );
        match received.message {
            Message::Report { notice, .. } => {
                if let Some(snapshot) = assembler.push(notice)? {
                    return Ok(Reply::Snapshot(snapshot));
                }
            }
            Message::Rejected { reason, .. } => return Ok(Reply::Rejected(reason)),
            _ => anyhow::bail!("unexpected broker control reply"),
        }
    }
}
