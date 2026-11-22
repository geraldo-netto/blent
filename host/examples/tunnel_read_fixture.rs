//! T1770 private loopback measurement, public keys, no device/session authority.
use anyhow::{ensure, Context, Result};
use blent::broker::transport::{Event, Transport};
use blent::secure_connection::{
    accept, HandshakeTimeout, Identity, IdentityRecord, PeerCertificate,
};
use blent_config::connection::{
    mux::Multiplexer,
    wire::{Channel, Kind, MAX_PAYLOAD},
};
use std::{future::poll_fn, io::Write, path::Path, time::Duration};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

const DEADLINE: Duration = Duration::from_secs(5);

struct Sample {
    size: u32,
    frames: usize,
    pipeline: usize,
}
impl Sample {
    fn new(size: u32, frames: usize, pipeline: usize) -> Result<Self> {
        ensure!((1..=MAX_PAYLOAD).contains(&size), "invalid payload size");
        ensure!((1..=60).contains(&frames), "invalid frame count");
        ensure!(
            (1..=4).contains(&pipeline) && pipeline <= frames,
            "invalid pipeline"
        );
        Ok(Self {
            size,
            frames,
            pipeline,
        })
    }
    fn mux(&self) -> Result<Multiplexer> {
        let mut mux = Multiplexer::new(
            1,
            1,
            self.size * self.pipeline as u32,
            self.pipeline,
            MAX_PAYLOAD,
        )?;
        // Synthetic fixture activation grants no resource/media authority.
        mux.activate(Channel::Display, 1)?;
        for _ in 0..self.pipeline {
            self.offer(&mut mux)?;
        }
        Ok(mux)
    }
    fn offer(&self, mux: &mut Multiplexer) -> Result<()> {
        ensure!(
            mux.offer(Channel::Display, vec![7; self.size as usize])
                .is_ok(),
            "fixture queue full"
        );
        Ok(())
    }
}

#[derive(Default)]
struct Progress {
    written: usize,
    credited: u32,
}
impl Progress {
    fn observe(&mut self, event: Event, mux: &mut Multiplexer, sample: &Sample) -> Result<bool> {
        match event {
            Event::Written(header) if header.kind == Kind::Data => {
                self.written += 1;
                if self.written + sample.pipeline <= sample.frames {
                    sample.offer(mux)?;
                }
            }
            Event::Written(_) => (),
            Event::Received { header, deliver } => {
                ensure!(
                    header.kind == Kind::Credit && !deliver,
                    "unexpected fixture input"
                );
                if header.channel == Channel::Display {
                    self.credited += header.credit;
                }
            }
        }
        Ok(self.written == sample.frames
            && self.credited == sample.size * (sample.frames + sample.pipeline) as u32)
    }
}

async fn framed<S: AsyncRead + AsyncWrite + Unpin>(stream: &mut S, sample: &Sample) -> Result<()> {
    let mut transport = Transport::new(stream, sample.mux()?, DEADLINE)?;
    let mut progress = Progress::default();
    for _ in 0..(2 * sample.frames + 8) {
        let event =
            tokio::time::timeout(DEADLINE, poll_fn(|cx| transport.poll_event(cx))).await??;
        let complete = progress.observe(event, transport.multiplexer(), sample)?;
        if matches!(event, Event::Received { .. }) {
            transport.consume()?;
        }
        if complete {
            return Ok(());
        }
    }
    anyhow::bail!("fixture event bound exceeded")
}

async fn finish<S: AsyncRead + Unpin>(stream: &mut S) -> Result<()> {
    let marker = tokio::time::timeout(DEADLINE, stream.read_u8()).await??;
    ensure!(marker == 127, "invalid fixture finish marker");
    let mut byte = [0];
    ensure!(
        tokio::time::timeout(DEADLINE, stream.read(&mut byte)).await?? == 0,
        "missing client EOF"
    );
    Ok(())
}

async fn serve(directory: &Path, ready: &Path, sample: &Sample) -> Result<()> {
    let identity = Identity::load(IdentityRecord {
        certificate: std::fs::read(directory.join("host.der"))?,
        private_key: std::fs::read(directory.join("host.pk8"))?,
    })?;
    let peer = PeerCertificate::parse(std::fs::read(directory.join("tablet.der"))?)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(ready)?;
    writeln!(file, "{}", listener.local_addr()?.port())?;
    drop(file);
    let (socket, address) = tokio::time::timeout(DEADLINE, listener.accept()).await??;
    ensure!(address.ip().is_loopback(), "non-loopback peer");
    socket.set_nodelay(true)?;
    let mut stream = accept(socket, &identity, &peer, HandshakeTimeout::new(DEADLINE)?).await?;
    framed(&mut stream, sample).await?;
    finish(&mut stream).await?;
    println!("settled {} frames", sample.frames);
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() == 6,
        "usage: tunnel_read_fixture PUBLIC_FIXTURES NEW_READY_FILE SIZE FRAMES PIPELINE"
    );
    let sample = Sample::new(
        args[3].parse().context("size")?,
        args[4].parse().context("frames")?,
        args[5].parse().context("pipeline")?,
    )?;
    serve(Path::new(&args[1]), Path::new(&args[2]), &sample).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1770_invalid_domains_are_rejected_before_native_admission() {
        for size in [0, MAX_PAYLOAD + 1, u32::MAX] {
            assert!(Sample::new(size, 16, 4).is_err());
        }
        for frames in [0, 61, usize::MAX] {
            assert!(Sample::new(64, frames, 4).is_err());
        }
        for pipeline in [0, 5, usize::MAX] {
            assert!(Sample::new(64, 16, pipeline).is_err());
        }
        assert!(Sample::new(64, 1, 4).is_err());
        assert!(Sample::new(MAX_PAYLOAD, 60, 4).unwrap().mux().is_ok());
    }
}
