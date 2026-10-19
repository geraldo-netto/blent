//! T846 physical TLS/multiplexer qualification, synthetic payloads only.
//! Not a production connection broker; no application tokens or media authority.
#[cfg(target_os = "linux")]
mod linux {
    use anyhow::{ensure, Context, Result};
    use blent::{
        accessory::{self, Resource, Stream},
        secure_connection::{Identity, IdentityRecord, PeerCertificate},
    };
    use blent_config::connection::{
        mux::Multiplexer,
        pairing::Pairing,
        wire::{Channel, Header, Kind, CHANNELS, HEADER_BYTES},
    };
    use std::{path::PathBuf, time::Duration};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    fn arguments() -> Result<(PathBuf, accessory::linux::Target)> {
        let args: Vec<_> = std::env::args().collect();
        ensure!(
            args.len() == 4,
            "usage: accessory_probe HELPER BUS PORT.PATH (changes USB mode)"
        );
        let ports = args[3]
            .split('.')
            .map(str::parse)
            .collect::<std::result::Result<Vec<u8>, _>>()?;
        Ok((
            PathBuf::from(&args[1]),
            accessory::linux::Target::new(args[2].parse()?, &ports)?,
        ))
    }
    fn fingerprint(identity: &Identity) -> Result<String> {
        Ok(PeerCertificate::parse(identity.certificate().to_vec())?
            .fingerprint()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }
    async fn console_approval(deadline: Duration) -> Result<()> {
        // A detached process-owned thread can block on console input. Tokio's
        // blocking pool would instead make runtime shutdown wait indefinitely.
        let (send, receive) = tokio::sync::oneshot::channel();
        std::thread::Builder::new()
            .name("accessory-approval".into())
            .spawn(move || {
                use std::io::{BufRead, Read};
                let mut text = String::new();
                let result = std::io::BufReader::new(std::io::stdin().take(32))
                    .read_line(&mut text)
                    .map(|_| text);
                let _ = send.send(result);
            })?;
        let text = tokio::time::timeout(deadline, receive).await???;
        ensure!(text.trim() == "approve", "pairing declined");
        Ok(())
    }
    async fn approved(
        candidate: &accessory::setup::Candidate,
        identity: &Identity,
    ) -> Result<blent_config::connection::pairing::Ticket> {
        println!("Computer fingerprint: {}", fingerprint(identity)?);
        println!("Tablet fingerprint: {}", candidate.peer().as_str());
        println!("Match Computer with Computer and Tablet with Tablet on the tablet; the two identities differ. Approve there, then type approve here. Synthetic test only; no persistent trust.");
        console_approval(Duration::from_secs(120)).await?;
        let mut pairing = Pairing::default();
        let ticket = pairing.begin(candidate.peer().clone(), false, 0, 120000)?;
        pairing.approve(&ticket, false, 1)?;
        Ok(ticket)
    }
    async fn flush(stream: &mut dyn Stream, mux: &mut Multiplexer) -> Result<()> {
        while let Some(frame) = mux.next_frame()? {
            let mut header = [0; HEADER_BYTES];
            frame.header.encode(&mut header)?;
            stream.write_all(&header).await?;
            stream.write_all(frame.bytes).await?;
            stream.flush().await?;
            mux.finish(true)?;
        }
        Ok(())
    }
    async fn receive(stream: &mut dyn Stream, mux: &mut Multiplexer) -> Result<(Header, Vec<u8>)> {
        let mut header = [0; HEADER_BYTES];
        stream.read_exact(&mut header).await?;
        let header = Header::decode(&header)?;
        let deliver = mux.receive(header)?;
        let mut bytes = vec![0; header.length as usize];
        stream.read_exact(&mut bytes).await?;
        ensure!(
            header.kind != Kind::Data || deliver,
            "unexpected retired payload"
        );
        Ok((header, bytes))
    }
    async fn exchange(stream: &mut dyn Stream) -> Result<()> {
        let mut mux = Multiplexer::new(7, 9, 1048576, 64, 65536)?;
        for id in 1..CHANNELS {
            mux.activate(Channel::try_from(id)?, 9)?;
        }
        flush(stream, &mut mux).await?;
        for _ in 0..CHANNELS {
            let (header, _) = receive(stream, &mut mux).await?;
            ensure!(header.kind == Kind::Credit, "initial credit missing");
        }
        println!("sequence,channel,payload_bytes,echo_roundtrip_ns");
        let start = std::time::Instant::now();
        for sequence in 0..4096usize {
            let channel = Channel::try_from((sequence % CHANNELS as usize + 1) as u8)?;
            let length = [1, 64, 512, 4096, 16384, 65536][(sequence / CHANNELS as usize) % 6];
            let bytes: Vec<u8> = (0..length)
                .map(|i| (i * 31 + sequence * 17) as u8)
                .collect();
            mux.offer(channel, bytes.clone())
                .map_err(|_| anyhow::anyhow!("outgoing queue full"))?;
            let before = std::time::Instant::now();
            flush(stream, &mut mux).await?;
            loop {
                let (header, echo) = receive(stream, &mut mux).await?;
                if header.kind != Kind::Data {
                    continue;
                }
                ensure!(
                    header.channel == channel && echo == bytes,
                    "echo integrity mismatch"
                );
                mux.release(channel, header.length)?;
                break;
            }
            println!(
                "{sequence},{},{length},{}",
                channel as u8,
                before.elapsed().as_nanos()
            );
        }
        mux.offer(Channel::Management, b"BLENTUSB-FINISH\x01".to_vec())
            .map_err(|_| anyhow::anyhow!("final receipt queue full"))?;
        flush(stream, &mut mux).await?;
        finish(stream, &mut mux).await?;
        eprintln!(
            "PASS: 4096 TLS-authenticated patterned echoes across six channels in {:.3}s",
            start.elapsed().as_secs_f64()
        );
        Ok(())
    }
    // The TLS close notification proves the Android peer consumed the final
    // receipt and drained its output. Native USB itself has no stream EOF.
    async fn finish(stream: &mut dyn Stream, mux: &mut Multiplexer) -> Result<()> {
        for _ in 0..=CHANNELS {
            let mut bytes = [0; HEADER_BYTES];
            if stream.read(&mut bytes[..1]).await? == 0 {
                return Ok(());
            }
            stream.read_exact(&mut bytes[1..]).await?;
            let header = Header::decode(&bytes)?;
            ensure!(header.kind == Kind::Credit, "unexpected final envelope");
            mux.receive(header)?;
        }
        anyhow::bail!("peer did not close authenticated test stream")
    }
    pub async fn run() -> Result<()> {
        if std::env::args().skip(1).eq(["--approval-check"]) {
            return console_approval(Duration::from_millis(100)).await;
        }
        let (helper, target) = arguments()?;
        let identity = Identity::load(IdentityRecord::generate()?)?;
        let resource = accessory::linux::open(&helper, &target)?;
        let candidate =
            accessory::setup::discover(resource, &identity, Duration::from_secs(60)).await?;
        let ticket = approved(&candidate, &identity).await?;
        let verified = candidate.authenticate(&identity, ticket).await?;
        let Resource { mut stream, owner } = verified.resource;
        let result = tokio::time::timeout(Duration::from_secs(60), exchange(&mut *stream))
            .await
            .context("physical trial deadline");
        drop(stream);
        owner.retire().await?;
        result??;
        Ok(())
    }
}
#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    linux::run().await
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Native accessory probe is supported only on Linux");
    std::process::exit(2);
}
