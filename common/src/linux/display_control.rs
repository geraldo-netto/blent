//! Linux transport for the independent display-control protocol. Abstract Unix
//! sockets leave no filesystem endpoints. Every connection is user/owner bound.
use crate::display_control::{Command, Reply, MAX_FRAME};
use anyhow::{ensure, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    task::JoinSet,
};

const IO_DEADLINE: Duration = Duration::from_secs(2);

fn endpoint(pid: u32) -> Result<(String, u64)> {
    let start = super::processes::start_ticks(pid).context("display owner exited")?;
    let uid = unsafe { libc::getuid() };
    Ok((format!("\0blent-display-{uid}-{pid}-{start}"), start))
}

fn peer(stream: &UnixStream, uid: u32, pid: Option<u32>) -> Result<()> {
    let credentials = stream.peer_cred()?;
    ensure!(credentials.uid() == uid, "display control user mismatch");
    if let Some(pid) = pid {
        ensure!(
            credentials.pid() == Some(pid as i32),
            "display control owner mismatch"
        );
    }
    Ok(())
}

async fn read<T: DeserializeOwned>(stream: &mut UnixStream) -> Result<T> {
    let mut bytes = Vec::new();
    stream
        .take((MAX_FRAME + 1) as u64)
        .read_to_end(&mut bytes)
        .await?;
    ensure!(
        bytes.len() <= MAX_FRAME,
        "display control frame exceeds capacity"
    );
    Ok(serde_json::from_slice(&bytes)?)
}

async fn write<T: Serialize>(stream: &mut UnixStream, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    ensure!(
        bytes.len() <= MAX_FRAME,
        "display control frame exceeds capacity"
    );
    stream.write_all(&bytes).await?;
    stream.shutdown().await?;
    Ok(())
}

pub struct Client {
    runtime: tokio::runtime::Runtime,
}
impl Client {
    pub fn new() -> Result<Self> {
        Ok(Self {
            runtime: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
        })
    }
    pub fn request(&self, pid: u32, command: &Command) -> Result<Reply> {
        command.validate()?;
        self.runtime.block_on(async {
            tokio::time::timeout(IO_DEADLINE, exchange(pid, command))
                .await
                .context(
                    "display command outcome unconfirmed; inspect current state before retry",
                )?
        })
    }
}

async fn exchange(pid: u32, command: &Command) -> Result<Reply> {
    let (address, start) = endpoint(pid)?;
    let mut stream = UnixStream::connect(address).await?;
    peer(&stream, unsafe { libc::getuid() }, Some(pid))?;
    ensure!(
        super::processes::start_ticks(pid) == Some(start),
        "display owner changed"
    );
    write(&mut stream, command).await?;
    let reply: Reply = read(&mut stream).await?;
    reply.snapshot.validate()?;
    Ok(reply)
}

pub struct Call {
    pub command: Command,
    stream: UnixStream,
}

pub struct Server {
    listener: UnixListener,
    jobs: JoinSet<Result<Option<Call>>>,
    capacity: usize,
}
impl Server {
    pub fn bind(capacity: usize) -> Result<Self> {
        ensure!(
            (1..=64).contains(&capacity),
            "display control capacity must be 1..64"
        );
        let (address, _) = endpoint(std::process::id())?;
        Ok(Self {
            listener: UnixListener::bind(address)?,
            jobs: JoinSet::new(),
            capacity,
        })
    }

    pub async fn next(&mut self) -> Result<Call> {
        loop {
            let accept = self.jobs.len() < self.capacity;
            tokio::select! {
                connection = self.listener.accept(), if accept => {
                    let (stream, _) = connection?;
                    self.jobs.spawn(receive(stream, unsafe { libc::getuid() }));
                }
                result = self.jobs.join_next(), if !self.jobs.is_empty() => {
                    if let Some(Ok(Ok(Some(call)))) = result { return Ok(call); }
                }
            }
        }
    }

    pub fn respond(&mut self, mut call: Call, reply: Reply) {
        self.jobs.spawn(async move {
            tokio::time::timeout(IO_DEADLINE, write(&mut call.stream, &reply)).await??;
            Ok(None)
        });
    }

    pub async fn close(mut self) {
        self.jobs.abort_all();
        while self.jobs.join_next().await.is_some() {}
    }
}

async fn receive(mut stream: UnixStream, uid: u32) -> Result<Option<Call>> {
    peer(&stream, uid, None)?;
    let command: Command = tokio::time::timeout(IO_DEADLINE, read(&mut stream)).await??;
    command.validate()?;
    Ok(Some(Call { command, stream }))
}

#[cfg(test)]
mod tests;
